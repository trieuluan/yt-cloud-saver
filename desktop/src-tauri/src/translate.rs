use crate::config::{load_config, AppConfig};
use crate::tools::{build_env_path, find_binary, whisper_model_path};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub original: String,
    pub translated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleBundle {
    pub video_id: String,
    #[serde(default)]
    pub title: String,
    pub target_lang: String,
    pub source: String, // "youtube-cc" | "whisper"
    #[serde(default)]
    pub partial: bool,
    pub cues: Vec<SubtitleCue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleSummary {
    pub file_name: String,
    pub video_id: String,
    pub title: String,
    pub target_lang: String,
    pub source: String,
    pub line_count: usize,
}

fn cache_path(data_dir: &Path, video_id: &str, target_lang: &str) -> PathBuf {
    let safe_lang: String = target_lang
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    data_dir
        .join("subtitles")
        .join(format!("{video_id}_{safe_lang}.json"))
}

fn raw_cache_path(data_dir: &Path, video_id: &str) -> PathBuf {
    data_dir.join("subtitles").join(format!("{video_id}_raw.json"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCuesCache {
    source: String,
    cues: Vec<SubtitleCue>,
}

fn load_raw_cache(data_dir: &Path, video_id: &str) -> Option<RawCuesCache> {
    let raw = fs::read_to_string(raw_cache_path(data_dir, video_id)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_raw_cache(data_dir: &Path, video_id: &str, source: &str, cues: &[SubtitleCue]) {
    let path = raw_cache_path(data_dir, video_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let cache = RawCuesCache {
        source: source.to_string(),
        cues: cues.to_vec(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cache) {
        let _ = fs::write(path, json);
    }
}

pub fn load_cached_subtitles(
    data_dir: &Path,
    video_id: &str,
    target_lang: &str,
) -> Option<SubtitleBundle> {
    let raw = fs::read_to_string(cache_path(data_dir, video_id, target_lang)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_cached_subtitles(data_dir: &Path, bundle: &SubtitleBundle) {
    let path = cache_path(data_dir, &bundle.video_id, &bundle.target_lang);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(bundle) {
        let _ = fs::write(path, json);
    }
}

fn parse_timestamp_ms(ts: &str) -> Option<u64> {
    let clean = ts.trim().split_whitespace().next()?.replace(',', ".");
    let parts: Vec<&str> = clean.split(':').collect();
    let (h, m, s_ms) = match parts.len() {
        3 => (
            parts[0].parse::<u64>().ok()?,
            parts[1].parse::<u64>().ok()?,
            parts[2],
        ),
        2 => (0, parts[0].parse::<u64>().ok()?, parts[1]),
        _ => return None,
    };
    let (s_str, ms_str) = s_ms.split_once('.').unwrap_or((s_ms, "0"));
    let s = s_str.parse::<u64>().ok()?;
    let ms_padded = format!("{:0<3}", &ms_str[..ms_str.len().min(3)]);
    let ms = ms_padded.parse::<u64>().ok()?;
    Some((h * 3600 + m * 60 + s) * 1000 + ms)
}

fn strip_vtt_tags(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_tag = false;
    for ch in line.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

fn parse_vtt_or_srt(content: &str) -> Vec<SubtitleCue> {
    let mut cues: Vec<SubtitleCue> = Vec::new();
    let normalized = content.replace("\r\n", "\n");
    for block in normalized.split("\n\n") {
        let lines: Vec<&str> = block.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        let Some(arrow_idx) = lines.iter().position(|l| l.contains("-->")) else {
            continue;
        };
        let timing = lines[arrow_idx];
        let Some((lhs, rhs)) = timing.split_once("-->") else {
            continue;
        };
        let (Some(start_ms), Some(end_ms)) = (parse_timestamp_ms(lhs), parse_timestamp_ms(rhs)) else {
            continue;
        };
        let text = lines[arrow_idx + 1..]
            .iter()
            .map(|l| strip_vtt_tags(l))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            continue;
        }
        // Deduplicate rolling YouTube auto-caption repeats
        if let Some(prev) = cues.last_mut() {
            if prev.original == text && start_ms <= prev.end_ms + 300 {
                prev.end_ms = prev.end_ms.max(end_ms);
                continue;
            }
        }
        cues.push(SubtitleCue {
            start_ms,
            end_ms: end_ms.max(start_ms + 500),
            original: text,
            translated: String::new(),
        });
    }
    cues
}

async fn extract_raw_cues<F>(
    data_dir: &Path,
    bin_dir: &Path,
    config: &AppConfig,
    url: &str,
    video_id: &str,
    on_progress: &F,
) -> Result<(String, Vec<SubtitleCue>), String>
where
    F: Fn(u8, String),
{
    let ytdlp = find_binary("yt-dlp", bin_dir)
        .ok_or_else(|| "yt-dlp is not installed".to_string())?;
    let tmp_dir = PathBuf::from(&config.temp_dir).join(format!("sub_{video_id}"));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

    let out_tmpl = tmp_dir.join("sub").to_string_lossy().to_string();
    let mut args = vec![
        "--skip-download".to_string(),
        "--no-playlist".to_string(),
        "--write-subs".to_string(),
    ];
    if !(config.prefer_whisper_over_auto_cc && config.use_whisper_fallback) {
        args.push("--write-auto-subs".to_string());
    }
    args.extend([
        "--sub-format".to_string(),
        "vtt".to_string(),
        "--sub-langs".to_string(),
        "en.*,en,ja,ko,zh.*,vi".to_string(),
        "-o".to_string(),
        out_tmpl,
    ]);
    if !config.cookies_browser.is_empty() {
        args.push("--cookies-from-browser".to_string());
        args.push(config.cookies_browser.clone());
    }
    args.push(url.to_string());

    let _ = Command::new(&ytdlp)
        .args(&args)
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await;

    let vtt_file = fs::read_dir(&tmp_dir)
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().and_then(|e| e.to_str()) == Some("vtt"))
        });

    if let Some(vtt_path) = vtt_file {
        let raw = fs::read_to_string(&vtt_path).unwrap_or_default();
        let cues = parse_vtt_or_srt(&raw);
        let _ = fs::remove_dir_all(&tmp_dir);
        if !cues.is_empty() {
            return Ok(("youtube-cc".to_string(), cues));
        }
    }

    if !config.use_whisper_fallback {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("No YouTube captions found (and whisper.cpp fallback is disabled).".to_string());
    }

    let whisper = find_binary("whisper-cli", bin_dir)
        .or_else(|| find_binary("whisper", bin_dir))
        .ok_or_else(|| {
            let _ = fs::remove_dir_all(&tmp_dir);
            "No YouTube captions found. Install whisper.cpp (brew install whisper-cpp) for audio transcription.".to_string()
        })?;

    let ffmpeg = find_binary("ffmpeg", bin_dir)
        .ok_or_else(|| "ffmpeg is required for whisper transcription".to_string())?;

    let audio_raw = tmp_dir.join("audio.m4a");
    let mut dl_args = vec![
        "--newline".to_string(),
        "--no-playlist".to_string(),
        "-f".to_string(),
        "ba[ext=m4a]/ba".to_string(),
        "-o".to_string(),
        audio_raw.to_string_lossy().to_string(),
    ];
    if !config.cookies_browser.is_empty() {
        dl_args.push("--cookies-from-browser".to_string());
        dl_args.push(config.cookies_browser.clone());
    }
    dl_args.push(url.to_string());

    on_progress(6, "Downloading audio...".to_string());
    let mut dl_child = Command::new(&ytdlp)
        .args(&dl_args)
        .env("PATH", build_env_path(bin_dir))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to download audio: {e}"))?;

    if let Some(stdout) = dl_child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(idx) = line.find("[download]") {
                if let Some(pct_idx) = line[idx + 10..].find('%') {
                    if let Some(num) = line[idx + 10..idx + 10 + pct_idx].trim().split_whitespace().last() {
                        if let Ok(val) = num.parse::<f64>() {
                            let p = 6 + ((val.clamp(0.0, 100.0) * 0.14) as u8);
                            on_progress(p, format!("Downloading audio {}%", val.round() as u8));
                        }
                    }
                }
            }
        }
    }

    let dl_out = dl_child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to download audio: {e}"))?;

    if !dl_out.status.success() || !audio_raw.is_file() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("Failed to download audio for Whisper".to_string());
    }

    on_progress(20, "Preparing audio chunks...".to_string());

    let seg_pattern = tmp_dir.join("chunk_%03d.wav");
    let ff_out = Command::new(ffmpeg)
        .args([
            "-y",
            "-i",
            &audio_raw.to_string_lossy(),
            "-ar",
            "16000",
            "-ac",
            "1",
            "-f",
            "segment",
            "-segment_time",
            "1800",
            &seg_pattern.to_string_lossy(),
        ])
        .output()
        .await
        .map_err(|e| format!("ffmpeg failed: {e}"))?;

    if !ff_out.status.success() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("Failed to convert audio to 16kHz WAV chunks".to_string());
    }
    let _ = fs::remove_file(&audio_raw);

    let model_name = if config.whisper_model.is_empty() {
        "small"
    } else {
        config.whisper_model.as_str()
    };
    let model_path = whisper_model_path(data_dir, model_name);
    if !model_path.is_file() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(format!(
            "Whisper model 'ggml-{model_name}.bin' is not downloaded. Go to AI & Translate to download it."
        ));
    }

    let mut chunk_files: Vec<PathBuf> = fs::read_dir(&tmp_dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("wav"))
        .collect();
    chunk_files.sort();

    if chunk_files.is_empty() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("No audio chunks produced by ffmpeg".to_string());
    }

    const CHUNK_MS: u64 = 1800 * 1000;
    let total_chunks = chunk_files.len();
    let mut translated_all: Vec<SubtitleCue> = Vec::new();
    let mut raw_all: Vec<SubtitleCue> = Vec::new();

    for (idx, chunk_wav) in chunk_files.iter().enumerate() {
        let base_pct = 20 + (((idx * 75) / total_chunks.max(1)) as u8);
        on_progress(
            base_pct,
            format!("Whisper chunk {}/{total_chunks} ({base_pct}%)", idx + 1),
        );
        let srt_prefix = tmp_dir.join(format!("whisper_out_{idx:03}"));
        let wh_out = Command::new(&whisper)
            .args([
                "-m",
                &model_path.to_string_lossy(),
                "-bs",
                "1",
                "-f",
                &chunk_wav.to_string_lossy(),
                "-osrt",
                "-of",
                &srt_prefix.to_string_lossy(),
            ])
            .env("PATH", build_env_path(bin_dir))
            .output()
            .await
            .map_err(|e| format!("whisper-cli failed: {e}"))?;

        let srt_path = tmp_dir.join(format!("whisper_out_{idx:03}.srt"));
        if !wh_out.status.success() || !srt_path.is_file() {
            let err = String::from_utf8_lossy(&wh_out.stderr).trim().to_string();
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(if err.is_empty() {
                "Whisper transcription failed".to_string()
            } else {
                err
            });
        }

        let srt_raw = fs::read_to_string(&srt_path).unwrap_or_default();
        let offset_ms = (idx as u64) * CHUNK_MS;
        let mut chunk_cues = Vec::new();
        for mut cue in parse_vtt_or_srt(&srt_raw) {
            cue.start_ms += offset_ms;
            cue.end_ms += offset_ms;
            chunk_cues.push(cue);
        }
        let _ = fs::remove_file(chunk_wav);
        raw_all.extend(chunk_cues.clone());
        save_raw_cache(data_dir, video_id, "whisper", &raw_all);

        let latest_cfg = load_config(data_dir);
        let tr_pct = 20 + ((((idx * 75) + 40) / total_chunks.max(1)) as u8).min(78);
        on_progress(
            tr_pct,
            format!("Translating chunk {}/{total_chunks} ({tr_pct}%)", idx + 1),
        );
        const BATCH_SIZE: usize = 100;
        for batch in chunk_cues.chunks_mut(BATCH_SIZE) {
            let lines: Vec<&str> = batch.iter().map(|c| c.original.as_str()).collect();
            let prev_context = if translated_all.is_empty() {
                String::new()
            } else {
                let tail_start = translated_all.len().saturating_sub(6);
                let pairs: Vec<String> = translated_all[tail_start..]
                    .iter()
                    .map(|c| format!("- \"{}\" => \"{}\"", c.original, c.translated))
                    .collect();
                format!(
                    "\nPrevious context (for consistent pronouns/tone, DO NOT include in output):\n{}\n",
                    pairs.join("\n")
                )
            };
            let prompt = format!(
                "Translate these {} movie subtitle lines into natural {} suitable for spoken dialogue (keep pronouns/tone consistent).{}\nReturn ONLY a JSON array of strings with the exact same length ({}) in the same order.\n\n{}",
                lines.len(),
                latest_cfg.target_lang,
                prev_context,
                lines.len(),
                serde_json::to_string(&lines).unwrap_or_default()
            );
            let reply = call_llm(&latest_cfg, &prompt).await?;
            if let Some(translated) = parse_json_string_array(&reply) {
                for (cue, tr) in batch.iter_mut().zip(translated.into_iter()) {
                    cue.translated = tr;
                }
            }
            for cue in batch.iter_mut() {
                if cue.translated.is_empty() {
                    cue.translated = cue.original.clone();
                }
                translated_all.push(cue.clone());
            }
            let partial_bundle = SubtitleBundle {
                video_id: video_id.to_string(),
                title: video_id.to_string(),
                target_lang: latest_cfg.target_lang.clone(),
                source: "whisper".to_string(),
                partial: true,
                cues: translated_all.clone(),
            };
            save_cached_subtitles(data_dir, &partial_bundle);
        }
    }

    let _ = fs::remove_dir_all(&tmp_dir);
    if raw_all.is_empty() {
        return Err("Whisper produced empty subtitles".to_string());
    }
    Ok(("whisper".to_string(), translated_all))
}

async fn call_llm(config: &AppConfig, prompt: &str) -> Result<String, String> {
    let delays = [1500_u64, 3000, 6000];
    let mut last_err = String::new();
    for attempt in 0..=delays.len() {
        match call_llm_once(config, prompt).await {
            Ok(text) => return Ok(text),
            Err(err) => {
                let retryable = err.contains("HTTP 503") || err.contains("HTTP 429");
                last_err = err;
                if retryable && attempt < delays.len() {
                    tokio::time::sleep(std::time::Duration::from_millis(delays[attempt])).await;
                    continue;
                }
                return Err(last_err);
            }
        }
    }
    Err(last_err)
}

fn active_ai_creds(config: &AppConfig) -> (String, String, String) {
    let prof = config.ai_profiles.get(&config.ai_provider);
    let base_url = prof
        .map(|p| p.base_url.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(config.ai_base_url.as_str())
        .to_string();
    let api_key = prof
        .map(|p| p.api_key.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(config.ai_api_key.as_str())
        .to_string();
    let model = prof
        .map(|p| p.model.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(config.ai_model.as_str())
        .to_string();
    (base_url, api_key, model)
}

async fn call_llm_once(config: &AppConfig, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let (cfg_base, cfg_key, cfg_model) = active_ai_creds(config);
    let model = if cfg_model.is_empty() {
        "gemini-3.8-flash"
    } else {
        cfg_model.as_str()
    };

    if config.ai_provider == "gemini" {
        if cfg_key.is_empty() {
            return Err("Gemini API Key is required (configure in AI & Translate)".to_string());
        }
        let endpoint = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={cfg_key}"
        );
        let resp = client
            .post(&endpoint)
            .json(&json!({
                "contents": [{ "parts": [{ "text": prompt }] }]
            }))
            .send()
            .await
            .map_err(|e| format!("Gemini request failed: {e}"))?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("Gemini HTTP {status}: {body}"));
        }
        let text = body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        return Ok(text);
    }

    // OpenAI-compatible (OpenAI, OpenRouter, Groq, Ollama, etc.)
    let base = if config.ai_provider == "ollama" && cfg_base.is_empty() {
        "http://localhost:11434/v1"
    } else if cfg_base.is_empty() {
        "https://api.openai.com/v1"
    } else {
        cfg_base.as_str()
    };
    let endpoint = format!("{}/chat/completions", base.trim_end_matches('/'));

    let mut req = client.post(&endpoint).json(&json!({
        "model": model,
        "messages": [
            { "role": "system", "content": "You are an expert movie subtitle translator. Follow output format instructions strictly." },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.3
    }));

    if !cfg_key.is_empty() {
        req = req.bearer_auth(&cfg_key);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("LLM request failed ({endpoint}): {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("LLM HTTP {status}: {body}"));
    }
    let text = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(text)
}

pub async fn test_ai_connection(config: &AppConfig) -> Result<String, String> {
    let reply = call_llm(config, "Reply with exactly: OK").await?;
    Ok(reply.trim().to_string())
}

pub async fn list_ai_models(config: &AppConfig) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let (cfg_base, cfg_key, _) = active_ai_creds(config);
    if config.ai_provider == "gemini" {
        if cfg_key.is_empty() {
            return Err("Enter Gemini API Key first".to_string());
        }
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={cfg_key}"
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Gemini models: {e}"))?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("Gemini HTTP {status}: {body}"));
        }
        let mut models: Vec<String> = body["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        m["supportedGenerationMethods"]
                            .as_array()
                            .map(|methods| methods.iter().any(|x| x.as_str() == Some("generateContent")))
                            .unwrap_or(false)
                    })
                    .filter_map(|m| m["name"].as_str())
                    .map(|n| n.strip_prefix("models/").unwrap_or(n).to_string())
                    .collect()
            })
            .unwrap_or_default();
        models.sort();
        return Ok(models);
    }

    let base = if config.ai_provider == "ollama" && cfg_base.is_empty() {
        "http://localhost:11434/v1"
    } else if cfg_base.is_empty() {
        "https://api.openai.com/v1"
    } else {
        cfg_base.as_str()
    };
    let url = format!("{}/models", base.trim_end_matches('/'));
    let mut req = client.get(&url);
    if !cfg_key.is_empty() {
        req = req.bearer_auth(&cfg_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models ({url}): {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {body}"));
    }
    let mut models: Vec<String> = body["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    Ok(models)
}

fn parse_json_string_array(raw: &str) -> Option<Vec<String>> {
    let trimmed = raw.trim();
    let start = trimmed.find('[')?;
    let end = trimmed.rfind(']')?;
    serde_json::from_str::<Vec<String>>(&trimmed[start..=end]).ok()
}

pub async fn translate_video_subtitles<F>(
    data_dir: &Path,
    bin_dir: &Path,
    config: &AppConfig,
    url: &str,
    video_id: &str,
    title: &str,
    on_progress: F,
) -> Result<SubtitleBundle, String>
where
    F: Fn(u8, String),
{
    if let Some(cached) = load_cached_subtitles(data_dir, video_id, &config.target_lang) {
        if !cached.partial {
            return Ok(cached);
        }
    }

    let (source, mut cues) = if let Some(raw) = load_raw_cache(data_dir, video_id) {
        on_progress(50, "Loaded cached transcription, resuming translation...".to_string());
        (raw.source, raw.cues)
    } else {
        on_progress(5, "Checking YouTube captions...".to_string());
        let (src, extracted) =
            extract_raw_cues(data_dir, bin_dir, config, url, video_id, &on_progress).await?;
        save_raw_cache(data_dir, video_id, &src, &extracted);
        (src, extracted)
    };

    // If extract_raw_cues already translated each chunk progressively, skip re-translating already translated cues
    let latest_cfg = load_config(data_dir);
    let already_done = cues.iter().all(|c| !c.translated.is_empty());
    if already_done && !cues.is_empty() {
        let bundle = SubtitleBundle {
            video_id: video_id.to_string(),
            title: title.to_string(),
            target_lang: latest_cfg.target_lang.clone(),
            source,
            partial: false,
            cues,
        };
        save_cached_subtitles(data_dir, &bundle);
        let _ = fs::remove_file(raw_cache_path(data_dir, video_id));
        return Ok(bundle);
    }

    let mut translated_all: Vec<SubtitleCue> = load_cached_subtitles(data_dir, video_id, &latest_cfg.target_lang)
        .filter(|b| b.partial && b.cues.len() < cues.len())
        .map(|b| b.cues)
        .unwrap_or_default();

    let resume_from = translated_all.len();
    let total_cues = cues.len().max(1);
    const BATCH_SIZE: usize = 100;
    let remaining = &mut cues[resume_from..];
    let total_batches = remaining.len().div_ceil(BATCH_SIZE).max(1);

    for (b_idx, chunk) in remaining.chunks_mut(BATCH_SIZE).enumerate() {
        let done_cues = resume_from + b_idx * BATCH_SIZE;
        let pct = 55 + (((done_cues * 40) / total_cues) as u8);
        on_progress(
            pct,
            format!("Translating batch {}/{total_batches} ({pct}%)", b_idx + 1),
        );
        let lines: Vec<&str> = chunk.iter().map(|c| c.original.as_str()).collect();
        let prev_context = if translated_all.is_empty() {
            String::new()
        } else {
            let tail_start = translated_all.len().saturating_sub(6);
            let pairs: Vec<String> = translated_all[tail_start..]
                .iter()
                .map(|c| format!("- \"{}\" => \"{}\"", c.original, c.translated))
                .collect();
            format!(
                "\nPrevious context (for consistent pronouns/tone, DO NOT include in output):\n{}\n",
                pairs.join("\n")
            )
        };
        let prompt = format!(
            "Translate these {} movie subtitle lines into natural {} suitable for spoken dialogue (keep pronouns/tone consistent for the video titled \"{}\").{}\nReturn ONLY a JSON array of strings with the exact same length ({}) in the same order.\n\n{}",
            lines.len(),
            latest_cfg.target_lang,
            title,
            prev_context,
            lines.len(),
            serde_json::to_string(&lines).unwrap_or_default()
        );
        let reply = call_llm(&latest_cfg, &prompt).await?;
        if let Some(translated) = parse_json_string_array(&reply) {
            for (cue, tr) in chunk.iter_mut().zip(translated.into_iter()) {
                cue.translated = tr;
            }
        }
        for cue in chunk.iter_mut() {
            if cue.translated.is_empty() {
                cue.translated = cue.original.clone();
            }
            translated_all.push(cue.clone());
        }

        // Persist progressive partial subtitles after every batch so playback & resume work seamlessly
        let is_last = translated_all.len() >= total_cues;
        let partial_bundle = SubtitleBundle {
            video_id: video_id.to_string(),
            title: title.to_string(),
            target_lang: latest_cfg.target_lang.clone(),
            source: source.clone(),
            partial: !is_last,
            cues: translated_all.clone(),
        };
        save_cached_subtitles(data_dir, &partial_bundle);
    }

    let bundle = SubtitleBundle {
        video_id: video_id.to_string(),
        title: title.to_string(),
        target_lang: latest_cfg.target_lang.clone(),
        source,
        partial: false,
        cues: translated_all,
    };
    save_cached_subtitles(data_dir, &bundle);
    let _ = fs::remove_file(raw_cache_path(data_dir, video_id));
    Ok(bundle)
}

pub fn list_cached_subtitles(data_dir: &Path) -> Vec<SubtitleSummary> {
    let dir = data_dir.join("subtitles");
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut out = vec![];
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&p) else {
            continue;
        };
        let Ok(bundle) = serde_json::from_str::<SubtitleBundle>(&raw) else {
            continue;
        };
        let file_name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if file_name.ends_with("_raw.json") {
            continue;
        }
        out.push(SubtitleSummary {
            file_name,
            title: if bundle.title.is_empty() {
                bundle.video_id.clone()
            } else {
                bundle.title
            },
            video_id: bundle.video_id,
            target_lang: bundle.target_lang,
            source: bundle.source,
            line_count: bundle.cues.len(),
        });
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    out
}

pub fn delete_cached_subtitle(data_dir: &Path, file_name: &str) -> Result<(), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid file name".to_string());
    }
    let path = data_dir.join("subtitles").join(file_name);
    if path.is_file() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn export_cached_srt(
    data_dir: &Path,
    file_name: &str,
    mode: Option<&str>,
) -> Result<String, String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid file name".to_string());
    }
    let path = data_dir.join("subtitles").join(file_name);
    let raw = fs::read_to_string(&path).map_err(|e| format!("Subtitle cache not found: {e}"))?;
    let bundle: SubtitleBundle = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let downloads = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Downloads"))
        .unwrap_or_else(std::env::temp_dir);
    let _ = fs::create_dir_all(&downloads);
    let safe_title: String = bundle
        .title
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .take(80)
        .collect();
    let base = if safe_title.trim().is_empty() {
        bundle.video_id.clone()
    } else {
        format!("{} [{}]", safe_title.trim(), bundle.video_id)
    };
    let export_mode = mode.unwrap_or("translated");
    let suffix = match export_mode {
        "original" => "original".to_string(),
        "bilingual" => format!("{}+orig", bundle.target_lang),
        _ => bundle.target_lang.clone(),
    };
    let dest = downloads.join(format!("{base}.{suffix}.srt"));
    write_srt_file_with_mode(&bundle, &dest, export_mode)?;
    Ok(dest.to_string_lossy().to_string())
}

fn format_srt_time(ms: u64) -> String {
    let total_s = ms / 1000;
    let rem_ms = ms % 1000;
    let h = total_s / 3600;
    let m = (total_s % 3600) / 60;
    let s = total_s % 60;
    format!("{h:02}:{m:02}:{s:02},{rem_ms:03}")
}

pub fn write_srt_file(bundle: &SubtitleBundle, dest: &Path) -> Result<(), String> {
    write_srt_file_with_mode(bundle, dest, "translated")
}

pub fn write_srt_file_with_mode(
    bundle: &SubtitleBundle,
    dest: &Path,
    mode: &str,
) -> Result<(), String> {
    let mut out = String::new();
    for (i, cue) in bundle.cues.iter().enumerate() {
        let text = match mode {
            "original" => cue.original.clone(),
            "bilingual" => {
                if cue.original.trim().is_empty() || cue.original == cue.translated {
                    cue.translated.clone()
                } else {
                    format!("{}\n{}", cue.translated, cue.original)
                }
            }
            _ => cue.translated.clone(),
        };
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            format_srt_time(cue.start_ms),
            format_srt_time(cue.end_ms),
            text
        ));
    }
    fs::write(dest, out).map_err(|e| e.to_string())
}

fn subtitle_backup_dest(rclone_remote: &str) -> String {
    let trimmed = rclone_remote.trim().trim_end_matches('/');
    if trimmed.ends_with(':') {
        format!("{trimmed}_ytcs_subtitles")
    } else {
        format!("{trimmed}/_ytcs_subtitles")
    }
}

pub async fn backup_subtitles_to_cloud(
    data_dir: &Path,
    bin_dir: &Path,
    rclone_remote: &str,
) -> Result<usize, String> {
    let subs_dir = data_dir.join("subtitles");
    let count = list_cached_subtitles(data_dir).len();
    if count == 0 {
        return Ok(0);
    }
    crate::tools::verify_remote_ready(bin_dir, rclone_remote).await?;
    let rclone = crate::tools::find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed".to_string())?;
    let dest = subtitle_backup_dest(rclone_remote);
    let out = tokio::process::Command::new(rclone)
        .env("PATH", crate::tools::build_env_path(bin_dir))
        .args([
            "copy",
            &subs_dir.to_string_lossy(),
            &dest,
            "--include",
            "*.json",
            "--exclude",
            "*_raw.json",
            "--update",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone backup: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(count)
}

pub async fn restore_subtitles_from_cloud(
    data_dir: &Path,
    bin_dir: &Path,
    rclone_remote: &str,
) -> Result<usize, String> {
    crate::tools::verify_remote_ready(bin_dir, rclone_remote).await?;
    let rclone = crate::tools::find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed".to_string())?;
    let subs_dir = data_dir.join("subtitles");
    let _ = fs::create_dir_all(&subs_dir);
    let src = subtitle_backup_dest(rclone_remote);
    let out = tokio::process::Command::new(rclone)
        .env("PATH", crate::tools::build_env_path(bin_dir))
        .args([
            "copy",
            &src,
            &subs_dir.to_string_lossy(),
            "--include",
            "*.json",
            "--exclude",
            "*_raw.json",
            "--update",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone restore: {e}"))?;
    if !out.status.success() {
        let err_txt = String::from_utf8_lossy(&out.stderr);
        if err_txt.to_lowercase().contains("not found") {
            return Ok(list_cached_subtitles(data_dir).len());
        }
        return Err(err_txt.trim().to_string());
    }
    Ok(list_cached_subtitles(data_dir).len())
}
