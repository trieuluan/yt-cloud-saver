use crate::config::{load_config, AppConfig};
use crate::tools::{build_env_path, find_binary, whisper_model_path};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
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
    #[serde(default)]
    partial: bool,
    #[serde(default)]
    completed_chunks: usize,
    #[serde(default)]
    total_chunks: usize,
    cues: Vec<SubtitleCue>,
}

fn load_raw_cache(data_dir: &Path, video_id: &str) -> Option<RawCuesCache> {
    let raw = fs::read_to_string(raw_cache_path(data_dir, video_id)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_raw_cache(
    data_dir: &Path,
    video_id: &str,
    source: &str,
    cues: &[SubtitleCue],
    partial: bool,
    completed_chunks: usize,
    total_chunks: usize,
) {
    let path = raw_cache_path(data_dir, video_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let cache = RawCuesCache {
        source: source.to_string(),
        partial,
        completed_chunks,
        total_chunks,
        cues: cues.to_vec(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cache) {
        let _ = fs::write(path, json);
    }
}

fn list_chunk_audio(tmp_dir: &Path, is_cloud: bool) -> Vec<PathBuf> {
    let want_ext = if is_cloud { "mp3" } else { "wav" };
    let mut files: Vec<PathBuf> = fs::read_dir(tmp_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(want_ext))
        .collect();
    files.sort();
    files
}

#[derive(Debug, Deserialize)]
struct CloudWhisperWord {
    start: f64,
    end: f64,
    #[serde(default)]
    #[allow(dead_code)]
    word: String,
}

#[derive(Debug, Deserialize)]
struct CloudWhisperSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Debug, Deserialize)]
struct CloudWhisperResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    segments: Vec<CloudWhisperSegment>,
    #[serde(default)]
    words: Vec<CloudWhisperWord>,
}

pub async fn test_whisper_cloud_connection(config: &AppConfig) -> Result<String, String> {
    let base = if config.whisper_cloud_base_url.trim().is_empty() {
        "https://api.groq.com/openai/v1"
    } else {
        config.whisper_cloud_base_url.trim().trim_end_matches('/')
    };
    let key = config.whisper_cloud_api_key.trim();
    if key.is_empty() {
        return Err("Whisper Cloud API Key is required".to_string());
    }
    let endpoint = format!("{base}/models");
    let res = reqwest::Client::new()
        .get(&endpoint)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {body}"));
    }
    let model = if config.whisper_cloud_model.trim().is_empty() {
        "whisper-large-v3-turbo"
    } else {
        config.whisper_cloud_model.trim()
    };
    Ok(format!("Connected ({model})"))
}

async fn transcribe_chunk_cloud(
    config: &AppConfig,
    chunk_path: &Path,
    whisper_lang: &str,
    offset_ms: u64,
) -> Result<Vec<SubtitleCue>, String> {
    let base = if config.whisper_cloud_base_url.trim().is_empty() {
        "https://api.groq.com/openai/v1"
    } else {
        config.whisper_cloud_base_url.trim().trim_end_matches('/')
    };
    let key = config.whisper_cloud_api_key.trim();
    if key.is_empty() {
        return Err("Whisper Cloud API Key is required (configure in AI & Translate)".to_string());
    }
    let model = if config.whisper_cloud_model.trim().is_empty() {
        "whisper-large-v3-turbo"
    } else {
        config.whisper_cloud_model.trim()
    };
    let bytes = fs::read(chunk_path).map_err(|e| format!("Failed to read audio chunk: {e}"))?;
    let file_name = chunk_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("chunk.mp3")
        .to_string();
    let endpoint = format!("{base}/audio/transcriptions");
    let client = reqwest::Client::new();
    let delays = [2000_u64, 4000, 8000];
    let mut last_err = String::new();

    for attempt in 0..=delays.len() {
        let part = reqwest::multipart::Part::bytes(bytes.clone())
            .file_name(file_name.clone())
            .mime_str("audio/mpeg")
            .map_err(|e| e.to_string())?;
        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", model.to_string())
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "segment")
            .text("timestamp_granularities[]", "word");
        if !whisper_lang.is_empty() && whisper_lang != "auto" {
            form = form.text("language", whisper_lang.to_string());
        }

        let res = client
            .post(&endpoint)
            .bearer_auth(key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if status.is_success() {
            let parsed: CloudWhisperResponse = serde_json::from_str(&body)
                .map_err(|e| format!("Invalid Whisper Cloud JSON: {e}"))?;
            let mut cues = Vec::new();
            for seg in parsed.segments {
                let text = seg.text.trim().to_string();
                if text.is_empty() {
                    continue;
                }
                let mut seg_start = seg.start.max(0.0);
                let mut seg_end = seg.end.max(seg_start + 0.2);
                if !parsed.words.is_empty() {
                    let seg_words: Vec<(f64, f64)> = parsed
                        .words
                        .iter()
                        .filter(|w| {
                            let mid = (w.start + w.end) * 0.5;
                            mid >= seg.start - 0.05 && mid <= seg.end + 0.05
                        })
                        .map(|w| (w.start.max(0.0), w.end.max(w.start)))
                        .collect();
                    if !seg_words.is_empty() {
                        // Split on internal gaps > 1.5s and keep the last dense spoken cluster
                        let mut cluster_start = seg_words[0].0;
                        let mut cluster_end = seg_words[0].1;
                        for win in seg_words.windows(2) {
                            if win[1].0 - win[0].1 > 1.5 {
                                cluster_start = win[1].0;
                            }
                            cluster_end = win[1].1;
                        }
                        seg_start = cluster_start.max(seg.start).max(0.0);
                        seg_end = cluster_end.min(seg.end).max(seg_start + 0.25);
                    }
                }
                let max_sec = estimate_max_cue_sec(&text);
                if seg_end - seg_start > max_sec + 1.0 {
                    seg_start = (seg_end - max_sec).max(seg_start);
                }
                let start_ms = offset_ms + (seg_start * 1000.0) as u64;
                let end_ms = offset_ms + (seg_end * 1000.0) as u64;
                cues.push(SubtitleCue {
                    start_ms,
                    end_ms,
                    original: text,
                    translated: String::new(),
                    speaker: None,
                });
            }
            if cues.is_empty() && !parsed.text.trim().is_empty() {
                cues.push(SubtitleCue {
                    start_ms: offset_ms,
                    end_ms: offset_ms + 10_000,
                    original: parsed.text.trim().to_string(),
                    translated: String::new(),
                    speaker: None,
                });
            }
            return Ok(cues);
        }

        last_err = format!("Whisper Cloud HTTP {status}: {body}");
        let retryable = status.as_u16() == 429 || status.as_u16() == 503;
        if retryable && attempt < delays.len() {
            tokio::time::sleep(std::time::Duration::from_millis(delays[attempt])).await;
            continue;
        }
        return Err(last_err);
    }
    Err(last_err)
}

fn parse_chunk_index(path: &Path) -> Option<usize> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_prefix("chunk_"))
        .and_then(|s| s.parse::<usize>().ok())
}

const TRANSLATE_BATCH_SIZE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexedSubtitleLine {
    i: usize,
    t: String,
}

#[derive(Default)]
struct TranslationMemory {
    glossary: Vec<(String, String)>,
    scene_note: String,
}

impl TranslationMemory {
    fn merge(&mut self, new_entries: BTreeMap<String, String>, new_scene: Option<String>) {
        for (k, v) in new_entries {
            let src = k.trim().to_string();
            let dst = v.trim().to_string();
            if src.is_empty() || dst.is_empty() {
                continue;
            }
            if let Some(existing) = self.glossary.iter_mut().find(|(s, _)| s == &src) {
                existing.1 = dst;
            } else if self.glossary.len() < 50 {
                self.glossary.push((src, dst));
            }
        }
        if let Some(sc) = new_scene {
            let clean = sc.trim().to_string();
            if !clean.is_empty() {
                self.scene_note = clean;
            }
        }
    }
}

#[derive(Deserialize)]
struct BatchTranslationObj {
    #[serde(default)]
    lines: Vec<IndexedSubtitleLine>,
    #[serde(default)]
    glossary: BTreeMap<String, String>,
    #[serde(default)]
    scene: Option<String>,
}

fn build_prev_context(translated_all: &[SubtitleCue], memory: &TranslationMemory) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !memory.glossary.is_empty() {
        let g_lines: Vec<String> = memory
            .glossary
            .iter()
            .map(|(k, v)| format!("- {k} => {v}"))
            .collect();
        parts.push(format!(
            "Global Character & Term Glossary (keep strictly consistent across the whole video):\n{}",
            g_lines.join("\n")
        ));
    }
    if !memory.scene_note.is_empty() {
        parts.push(format!("Current Scene & Pronouns: {}", memory.scene_note));
    }
    if !translated_all.is_empty() {
        let tail_start = translated_all.len().saturating_sub(4);
        let pairs: Vec<String> = translated_all[tail_start..]
            .iter()
            .map(|c| format!("- \"{}\" => \"{}\"", c.original, c.translated))
            .collect();
        parts.push(format!(
            "Previous {} lines (match immediate dialogue flow; DO NOT include in output):\n{}",
            pairs.len(),
            pairs.join("\n")
        ));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", parts.join("\n\n"))
    }
}

fn cleanup_whisper_cues(cues: &mut Vec<SubtitleCue>) {
    if cues.is_empty() {
        return;
    }
    cues.sort_by_key(|c| c.start_ms);
    let mut cleaned: Vec<SubtitleCue> = Vec::with_capacity(cues.len());
    for mut cur in cues.drain(..) {
        let cur_text = cur.original.trim();
        if cur_text.is_empty() {
            continue;
        }
        cur.original = cur_text.to_string();
        if let Some(prev) = cleaned.last_mut() {
            // Merge only true Whisper repetition glitches (overlapping timestamps or <250ms apart),
            // preserving genuine repeated dialogue like "không chịu đâu, không chịu đâu".
            if prev.original == cur.original
                && (cur.start_ms < prev.end_ms || cur.start_ms.saturating_sub(prev.start_ms) < 250)
            {
                prev.end_ms = prev.end_ms.max(cur.end_ms);
                continue;
            }
            if cur.start_ms < prev.end_ms {
                if cur.start_ms > prev.start_ms.saturating_add(120) {
                    prev.end_ms = cur.start_ms;
                } else {
                    cur.start_ms = prev.end_ms;
                }
            }
        }
        if cur.end_ms < cur.start_ms.saturating_add(150) {
            cur.end_ms = cur.start_ms.saturating_add(150);
        }
        cleaned.push(cur);
    }
    *cues = cleaned;
}

fn contains_cjk_or_kana(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            c as u32,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3040..=0x30FF | 0xAC00..=0xD7AF
        )
    })
}

fn estimate_max_cue_sec(text: &str) -> f64 {
    let char_count = text.chars().filter(|c| !c.is_whitespace()).count();
    if contains_cjk_or_kana(text) {
        (char_count as f64 * 0.35 + 1.5).clamp(2.5, 9.0)
    } else {
        (char_count as f64 * 0.14 + 1.8).clamp(2.8, 9.0)
    }
}

fn target_allows_cjk(target_lang: &str) -> bool {
    let low = target_lang.to_lowercase();
    low.contains("chinese")
        || low.contains("japanese")
        || low.contains("korean")
        || low == "zh"
        || low == "ja"
        || low == "ko"
}

fn needs_translation_retry(original: &str, translated: &str, target_lang: &str) -> bool {
    let tr = translated.trim();
    if tr.is_empty() {
        return true;
    }
    if !target_allows_cjk(target_lang) && contains_cjk_or_kana(tr) {
        return true;
    }
    let orig = original.trim();
    if tr == orig && (contains_cjk_or_kana(orig) || orig.chars().count() > 4) {
        return true;
    }
    false
}

async fn translate_indexed_subset(
    cfg: &AppConfig,
    items: &[IndexedSubtitleLine],
    prev_context: &str,
    title: Option<&str>,
    strict_retry: bool,
    memory: Option<&mut TranslationMemory>,
) -> Result<Vec<(usize, String)>, String> {
    let title_ctx = title
        .filter(|t| !t.is_empty())
        .map(|t| format!(" for the video titled \"{}\"", t))
        .unwrap_or_default();
    let retry_note = if strict_retry {
        format!(
            " CRITICAL: Translate EVERY single line into {} — do NOT copy or leave source-language text.",
            cfg.target_lang
        )
    } else {
        String::new()
    };
    let output_rule = if memory.is_some() {
        "4. Return ONLY a JSON object: {\"lines\":[{\"i\":0,\"t\":\"...\"},...],\"glossary\":{\"source_term\":\"translated_name\"},\"scene\":\"1-sentence current scene & pronouns\"}. In \"glossary\", ONLY include NEW character names, proper nouns, or ASR homophone aliases discovered in this batch (leave {} if none). Keep \"lines\" with exact same \"i\" indices; do NOT merge, split, or skip lines."
    } else {
        "4. Return ONLY a JSON array of objects with the exact same \"i\" indices and translated \"t\" strings: [{\"i\":0,\"t\":\"...\"}, ...]. Do NOT merge, split, or skip lines."
    };
fn strip_speaker_hint_prefix(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let tag = &rest[..end];
            if tag.starts_with("Nam") || tag.starts_with("Nữ") || tag.starts_with("Speaker") {
                return rest[end + 1..].trim().to_string();
            }
        }
    }
    t.to_string()
}

    let prompt = format!(
        "Translate these {} subtitle lines into natural, idiomatic {} for spoken movie/drama dialogue{}.\nRules:\n1. Use natural everyday spoken {} (avoid stiff literal/Hán-Việt phrasing or word-by-word translation). Prefer concise phrasing that fits natural dubbing cadence when possible without losing meaning or emotional nuance. Add natural punctuation ('?' for questions, '!' for exclamations/commands/calls, ',' for pauses) so TTS intonation sounds expressive, but do NOT force a trailing '.' on short incomplete fragments.\n2. Keep character names, relationships, and pronouns strictly consistent with the Global Glossary and scene context (e.g., couples use Anh–Em, friends/rivals stay consistent — never randomly switch to Tôi–Cậu). If a line starts with an audio speaker hint like \"[Nữ · A]\" or \"[Nam · B]\", use it to infer speaker gender and pronouns (prefer story context if they conflict), and NEVER include the \"[...]\" tag in your translated output.\n3. The source comes from speech recognition (ASR) and may contain homophone typos (e.g. 陈默 misheard as 沉默, 风唇→丰唇, 不生字→不识字). ONLY convert a homophone into a character name from the Global Glossary when it acts as a person's name/vocative/subject (standing alone, called at start/end of line like \"沉默，过来\", or before a person-only verb like \"沉默张嘴\"). Keep its literal meaning when it is a normal verb/adjective/state (e.g. \"他沉默了\" = \"Anh ấy im lặng\").\n{}{}{}\n\n{}",
        items.len(),
        cfg.target_lang,
        title_ctx,
        cfg.target_lang,
        output_rule,
        prev_context,
        retry_note,
        serde_json::to_string(items).unwrap_or_default()
    );
    let reply = call_llm(cfg, &prompt).await?;
    let trimmed = reply.trim();
    if let (Some(obj_start), Some(obj_end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        let arr_start = trimmed.find('[');
        if arr_start.map(|a| obj_start < a).unwrap_or(true) {
            let slice = &trimmed[obj_start..=obj_end];
            if let Ok(parsed) = serde_json::from_str::<BatchTranslationObj>(slice) {
                if !parsed.lines.is_empty() {
                    if let Some(mem) = memory {
                        mem.merge(parsed.glossary, parsed.scene);
                    }
                    return Ok(parsed
                        .lines
                        .into_iter()
                        .filter_map(|it| {
                            let t = strip_speaker_hint_prefix(&it.t);
                            if t.is_empty() { None } else { Some((it.i, t)) }
                        })
                        .collect());
                }
            }
        }
    }
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        let slice = &trimmed[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<IndexedSubtitleLine>>(slice) {
            return Ok(parsed
                .into_iter()
                .filter_map(|it| {
                    let t = it.t.trim().to_string();
                    if t.is_empty() { None } else { Some((it.i, t)) }
                })
                .collect());
        }
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(slice) {
            if arr.len() == items.len() {
                return Ok(items
                    .iter()
                    .zip(arr.into_iter())
                    .filter_map(|(it, s)| {
                        let t = s.trim().to_string();
                        if t.is_empty() { None } else { Some((it.i, t)) }
                    })
                    .collect());
            }
        }
    }
    Ok(Vec::new())
}

async fn translate_batch_exact(
    cfg: &AppConfig,
    batch: &mut [SubtitleCue],
    prev_context: &str,
    title: Option<&str>,
    memory: &mut TranslationMemory,
    speaker_hints: Option<&[String]>,
) -> Result<(), String> {
    let items: Vec<IndexedSubtitleLine> = batch
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let hint = speaker_hints.and_then(|h| h.get(i)).map(|s| s.as_str()).unwrap_or("");
            let text = if hint.is_empty() {
                c.original.clone()
            } else {
                format!("{hint} {}", c.original)
            };
            IndexedSubtitleLine { i, t: text }
        })
        .collect();

    if let Ok(pairs) = translate_indexed_subset(cfg, &items, prev_context, title, false, Some(memory)).await {
        for (idx, tr) in pairs {
            if idx < batch.len() {
                batch[idx].translated = tr;
            }
        }
    } else {
        // Propagate hard API errors (e.g. 429 / auth)
        let _ = translate_indexed_subset(cfg, &items, prev_context, title, false, Some(memory)).await?;
    }

    for round in 0..2 {
        let missing: Vec<IndexedSubtitleLine> = batch
            .iter()
            .enumerate()
            .filter(|(_, c)| needs_translation_retry(&c.original, &c.translated, &cfg.target_lang))
            .map(|(i, c)| IndexedSubtitleLine {
                i,
                t: c.original.clone(),
            })
            .collect();

        if missing.is_empty() {
            break;
        }
        let max_chunk = if round == 0 { 15 } else { 8 };
        let chunk_size = missing.len().min(max_chunk).max(1);
        for sub in missing.chunks(chunk_size) {
            if let Ok(pairs) = translate_indexed_subset(cfg, sub, prev_context, title, true, None).await {
                for (idx, tr) in pairs {
                    if idx < batch.len() && !tr.trim().is_empty() {
                        batch[idx].translated = tr;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn translate_cue_batches(
    cues: &mut [SubtitleCue],
    translated_all: &mut Vec<SubtitleCue>,
    data_dir: &Path,
    video_id: &str,
    latest_cfg: &AppConfig,
    speaker_hints: Option<&[String]>,
) -> Result<(), String> {
    let mut memory = TranslationMemory::default();
    for (b_idx, batch) in cues.chunks_mut(TRANSLATE_BATCH_SIZE).enumerate() {
        let prev_context = build_prev_context(translated_all, &memory);
        let hint_slice = speaker_hints.map(|all| {
            let start = b_idx * TRANSLATE_BATCH_SIZE;
            let end = (start + batch.len()).min(all.len());
            &all[start.min(all.len())..end]
        });
        translate_batch_exact(latest_cfg, batch, &prev_context, None, &mut memory, hint_slice).await?;
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
    Ok(())
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
            speaker: None,
        });
    }
    cues
}

fn is_suspicious_whisper_cue(cue: &SubtitleCue) -> bool {
    let dur_ms = cue.end_ms.saturating_sub(cue.start_ms);
    let chars = cue.original.trim().chars().count();
    dur_ms >= 15_000 && chars <= 8
}

fn find_hallucination_ranges(cues: &[SubtitleCue]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut i = 0usize;
    while i < cues.len() {
        let mut j = i + 1;
        while j < cues.len() && cues[j].original.trim() == cues[i].original.trim() {
            j += 1;
        }
        if j - i >= 3 {
            ranges.push((i, j));
            i = j;
            continue;
        }
        if is_suspicious_whisper_cue(&cues[i]) {
            let mut k = i + 1;
            while k < cues.len() && is_suspicious_whisper_cue(&cues[k]) {
                k += 1;
            }
            if k - i >= 2 {
                ranges.push((i, k));
                i = k;
                continue;
            }
        }
        i += 1;
    }
    ranges
}

async fn repair_whisper_chunk_cues(
    cues: Vec<SubtitleCue>,
    chunk_wav: &Path,
    offset_ms: u64,
    tmp_dir: &Path,
    idx: usize,
    whisper: &Path,
    ffmpeg: &Path,
    model_path: &Path,
    whisper_lang: &str,
    bin_dir: &Path,
) -> Vec<SubtitleCue> {
    let bad_ranges = find_hallucination_ranges(&cues);
    if bad_ranges.is_empty() {
        return cues;
    }

    let mut out: Vec<SubtitleCue> = Vec::with_capacity(cues.len());
    let mut cursor = 0usize;

    for (r_idx, (start_i, end_i)) in bad_ranges.into_iter().enumerate() {
        out.extend_from_slice(&cues[cursor..start_i]);

        let rel_start_ms = cues[start_i].start_ms.saturating_sub(offset_ms);
        let rel_end_ms = cues[end_i - 1].end_ms.saturating_sub(offset_ms);
        let dur_ms = rel_end_ms.saturating_sub(rel_start_ms);

        let mut repaired: Vec<SubtitleCue> = Vec::new();
        if dur_ms >= 2000 && dur_ms <= 900_000 {
            let clip_wav = tmp_dir.join(format!("repair_{idx}_{r_idx}.wav"));
            let start_s = format!("{:.3}", rel_start_ms as f64 / 1000.0);
            let dur_s = format!("{:.3}", dur_ms as f64 / 1000.0);
            let ff_ok = Command::new(ffmpeg)
                .args([
                    "-y",
                    "-ss",
                    &start_s,
                    "-t",
                    &dur_s,
                    "-i",
                    &chunk_wav.to_string_lossy(),
                    "-af",
                    "highpass=f=120,loudnorm",
                    "-ar",
                    "16000",
                    "-ac",
                    "1",
                    &clip_wav.to_string_lossy(),
                ])
                .env("PATH", build_env_path(bin_dir))
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if ff_ok && clip_wav.is_file() {
                let rep_prefix = tmp_dir.join(format!("repair_out_{idx}_{r_idx}"));
                let wh_ok = Command::new(whisper)
                    .args([
                        "-m",
                        &model_path.to_string_lossy(),
                        "-l",
                        whisper_lang,
                        "-mc",
                        "0",
                        "-bs",
                        "5",
                        "-et",
                        "2.4",
                        "-f",
                        &clip_wav.to_string_lossy(),
                        "-osrt",
                        "-of",
                        &rep_prefix.to_string_lossy(),
                    ])
                    .env("PATH", build_env_path(bin_dir))
                    .output()
                    .await
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                let _ = fs::remove_file(&clip_wav);
                let rep_srt = tmp_dir.join(format!("repair_out_{idx}_{r_idx}.srt"));
                if wh_ok && rep_srt.is_file() {
                    let raw = fs::read_to_string(&rep_srt).unwrap_or_default();
                    let _ = fs::remove_file(&rep_srt);
                    for mut c in parse_vtt_or_srt(&raw) {
                        c.start_ms += offset_ms + rel_start_ms;
                        c.end_ms += offset_ms + rel_start_ms;
                        repaired.push(c);
                    }
                }
            }
        }

        if !find_hallucination_ranges(&repaired).is_empty() {
            if let Some(first) = cues.get(start_i) {
                out.push(first.clone());
            }
        } else if !repaired.is_empty() {
            out.extend(repaired);
        } else if let Some(first) = cues.get(start_i) {
            out.push(first.clone());
        }

        cursor = end_i;
    }

    out.extend_from_slice(&cues[cursor..]);
    out
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
    let existing_raw = load_raw_cache(data_dir, video_id)
        .filter(|r| r.partial && r.source == "whisper");
    if existing_raw.is_none() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

    if existing_raw.is_none() {
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
    }

    if !config.use_whisper_fallback {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("No YouTube captions found (and whisper.cpp fallback is disabled).".to_string());
    }

    let is_cloud = config.whisper_mode == "cloud";
    if is_cloud && config.whisper_cloud_api_key.trim().is_empty() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("Whisper Cloud API Key is required (configure in AI & Translate)".to_string());
    }

    let whisper = if is_cloud {
        None
    } else {
        Some(
            find_binary("whisper-cli", bin_dir)
                .or_else(|| find_binary("whisper", bin_dir))
                .ok_or_else(|| {
                    let _ = fs::remove_dir_all(&tmp_dir);
                    "No YouTube captions found. Install whisper.cpp (brew install whisper-cpp) for audio transcription.".to_string()
                })?,
        )
    };

    let ffmpeg = find_binary("ffmpeg", bin_dir)
        .ok_or_else(|| "ffmpeg is required for whisper transcription".to_string())?;
    let mut chunk_files = if existing_raw.is_some() {
        list_chunk_audio(&tmp_dir, is_cloud)
    } else {
        Vec::new()
    };

    let seg_seconds = if is_cloud { "1200" } else { "1800" };
    let seg_pattern = tmp_dir.join(if is_cloud { "chunk_%03d.mp3" } else { "chunk_%03d.wav" });

    if chunk_files.is_empty() {
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

        let mut drive_fallback_ok = false;

        if dl_out.status.success() && audio_raw.is_file() {
            on_progress(20, "Preparing audio chunks...".to_string());
            let mut ff_args = vec![
                "-y".to_string(),
                "-i".to_string(),
                audio_raw.to_string_lossy().to_string(),
                "-vn".to_string(),
                "-ar".to_string(),
                "16000".to_string(),
                "-ac".to_string(),
                "1".to_string(),
            ];
            if is_cloud {
                ff_args.extend(["-b:a".to_string(), "48k".to_string()]);
            }
            ff_args.extend([
                "-f".to_string(),
                "segment".to_string(),
                "-segment_time".to_string(),
                seg_seconds.to_string(),
                seg_pattern.to_string_lossy().to_string(),
            ]);
            let ff_out = Command::new(&ffmpeg)
                .args(&ff_args)
                .output()
                .await
                .map_err(|e| format!("ffmpeg failed: {e}"))?;

            let _ = fs::remove_file(&audio_raw);
            if !ff_out.status.success() {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Err("Failed to convert audio chunks".to_string());
            }
        } else {
            let _ = fs::remove_file(&audio_raw);
            let ytdlp_err = String::from_utf8_lossy(&dl_out.stderr);
            let ytdlp_detail = ytdlp_err
                .lines()
                .rev()
                .find(|l| l.contains("ERROR:"))
                .or_else(|| ytdlp_err.lines().rev().find(|l| !l.trim().is_empty()))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Failed to download audio for Whisper".to_string());

            // Fallback: if this video is already saved on Google Drive, extract audio from local stream
            let stream_url = format!("http://127.0.0.1:{}/stream/{}", config.port, video_id);
            on_progress(15, "YouTube audio blocked, trying Google Drive stream...".to_string());
            let mut ff_args = vec![
                "-y".to_string(),
                "-i".to_string(),
                stream_url,
                "-vn".to_string(),
                "-ar".to_string(),
                "16000".to_string(),
                "-ac".to_string(),
                "1".to_string(),
            ];
            if is_cloud {
                ff_args.extend(["-b:a".to_string(), "48k".to_string()]);
            }
            ff_args.extend([
                "-f".to_string(),
                "segment".to_string(),
                "-segment_time".to_string(),
                seg_seconds.to_string(),
                seg_pattern.to_string_lossy().to_string(),
            ]);
            if let Ok(ff_out) = Command::new(&ffmpeg).args(&ff_args).output().await {
                if ff_out.status.success() && !list_chunk_audio(&tmp_dir, is_cloud).is_empty() {
                    drive_fallback_ok = true;
                }
            }

            if !drive_fallback_ok {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Err(ytdlp_detail);
            }
        }

        chunk_files = list_chunk_audio(&tmp_dir, is_cloud);
        if let Some(ref raw) = existing_raw {
            chunk_files.retain(|p| {
                let keep = parse_chunk_index(p).unwrap_or(0) >= raw.completed_chunks;
                if !keep {
                    let _ = fs::remove_file(p);
                }
                keep
            });
        }
    } else {
        on_progress(20, "Resuming Whisper from remaining audio chunks...".to_string());
    }

    let model_name = if config.whisper_model.is_empty() {
        "small"
    } else {
        config.whisper_model.as_str()
    };
    let model_path = whisper_model_path(data_dir, model_name);
    if !is_cloud && !model_path.is_file() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(format!(
            "Whisper model 'ggml-{model_name}.bin' is not downloaded. Go to AI & Translate to download it."
        ));
    }

    if chunk_files.is_empty() && existing_raw.is_none() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("No audio chunks produced by ffmpeg".to_string());
    }

    let chunk_ms: u64 = if is_cloud { 1200 * 1000 } else { 1800 * 1000 };
    let total_chunks = existing_raw
        .as_ref()
        .map(|r| r.total_chunks)
        .filter(|&n| n > 0)
        .unwrap_or(chunk_files.len())
        .max(chunk_files.len());
    let mut raw_all: Vec<SubtitleCue> = existing_raw
        .as_ref()
        .map(|r| r.cues.clone())
        .unwrap_or_default();
    let mut translated_all: Vec<SubtitleCue> = load_cached_subtitles(data_dir, video_id, &config.target_lang)
        .filter(|b| b.partial)
        .map(|b| b.cues)
        .unwrap_or_default();

    if translated_all.len() < raw_all.len() {
        let latest_cfg = load_config(data_dir);
        let start_idx = translated_all.len();
        on_progress(22, "Resuming translation of already transcribed chunk...".to_string());
        translate_cue_batches(
            &mut raw_all[start_idx..],
            &mut translated_all,
            data_dir,
            video_id,
            &latest_cfg,
            None,
        )
        .await?;
    }

    let whisper_lang = if config.whisper_language.trim().is_empty() {
        "auto"
    } else {
        config.whisper_language.trim()
    };
    let vad_model = if !is_cloud {
        crate::tools::ensure_whisper_vad_model(data_dir).await.ok()
    } else {
        None
    };

    let mut speaker_tracker = crate::diarize::SpeakerTracker::new();

    for (idx, chunk_wav) in chunk_files.iter().enumerate() {
        let chunk_idx = parse_chunk_index(chunk_wav).unwrap_or(idx);
        let base_pct = 20 + (((chunk_idx * 75) / total_chunks.max(1)) as u8);
        let mode_label = if is_cloud { "Whisper Cloud" } else { "Whisper" };
        on_progress(
            base_pct,
            format!("{mode_label} chunk {}/{total_chunks} ({base_pct}%)", chunk_idx + 1),
        );
        let offset_ms = (chunk_idx as u64) * chunk_ms;

        if config.skip_music_whisper {
            if crate::yamnet::ensure_yamnet_model(data_dir).await.is_ok() {
                let d_dir = data_dir.to_path_buf();
                let w_path = chunk_wav.to_path_buf();
                let sens = config.yamnet_sensitivity.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    crate::yamnet::filter_music_and_singing_wav(&d_dir, &w_path, &sens)
                })
                .await;
            }
        }

        let mut chunk_cues = if is_cloud {
            transcribe_chunk_cloud(config, chunk_wav, whisper_lang, offset_ms).await?
        } else {
            let whisper_bin = whisper.as_ref().ok_or("whisper-cli missing")?;
            let srt_prefix = tmp_dir.join(format!("whisper_out_{chunk_idx:03}"));
            let mut wh_args = vec![
                "-m".to_string(),
                model_path.to_string_lossy().to_string(),
                "-l".to_string(),
                whisper_lang.to_string(),
                "-mc".to_string(),
                "0".to_string(),
                "-et".to_string(),
                "2.4".to_string(),
                "-bs".to_string(),
                "1".to_string(),
            ];
            if let Some(ref vad_path) = vad_model {
                wh_args.push("--vad".to_string());
                wh_args.push("-vm".to_string());
                wh_args.push(vad_path.to_string_lossy().to_string());
            }
            wh_args.extend([
                "-f".to_string(),
                chunk_wav.to_string_lossy().to_string(),
                "-osrt".to_string(),
                "-of".to_string(),
                srt_prefix.to_string_lossy().to_string(),
            ]);
            let wh_out = Command::new(whisper_bin)
                .args(&wh_args)
                .env("PATH", build_env_path(bin_dir))
                .output()
                .await
                .map_err(|e| format!("whisper-cli failed: {e}"))?;

            let srt_path = tmp_dir.join(format!("whisper_out_{chunk_idx:03}.srt"));
            if !wh_out.status.success() || !srt_path.is_file() {
                let err = String::from_utf8_lossy(&wh_out.stderr).trim().to_string();
                return Err(if err.is_empty() {
                    "Whisper transcription failed".to_string()
                } else {
                    err
                });
            }

            let srt_raw = fs::read_to_string(&srt_path).unwrap_or_default();
            let mut raw_chunk_cues = Vec::new();
            for mut cue in parse_vtt_or_srt(&srt_raw) {
                cue.start_ms += offset_ms;
                cue.end_ms += offset_ms;
                let max_ms = (estimate_max_cue_sec(&cue.original) * 1000.0) as u64;
                if cue.end_ms.saturating_sub(cue.start_ms) > max_ms + 1000 {
                    cue.start_ms = cue.end_ms.saturating_sub(max_ms).max(cue.start_ms);
                }
                raw_chunk_cues.push(cue);
            }
            repair_whisper_chunk_cues(
                raw_chunk_cues,
                chunk_wav,
                offset_ms,
                &tmp_dir,
                chunk_idx,
                whisper_bin,
                &ffmpeg,
                &model_path,
                whisper_lang,
                bin_dir,
            )
            .await
        };
        cleanup_whisper_cues(&mut chunk_cues);

        let latest_cfg = load_config(data_dir);
        let speaker_hints = if latest_cfg.use_speaker_diarization
            && crate::diarize::ensure_wespeaker_model(data_dir).await.is_ok()
        {
            if let Ok(pairs) = speaker_tracker.diarize_chunk(data_dir, chunk_wav, offset_ms, &chunk_cues) {
                let mut hints = Vec::with_capacity(pairs.len());
                for (i, (hint, code)) in pairs.into_iter().enumerate() {
                    if i < chunk_cues.len() {
                        chunk_cues[i].speaker = code;
                    }
                    hints.push(hint);
                }
                Some(hints)
            } else {
                None
            }
        } else {
            None
        };
        let _ = fs::remove_file(chunk_wav);

        raw_all.extend(chunk_cues.clone());
        let completed_chunks = chunk_idx + 1;
        save_raw_cache(
            data_dir,
            video_id,
            "whisper",
            &raw_all,
            completed_chunks < total_chunks,
            completed_chunks,
            total_chunks,
        );

        let tr_pct = 20 + ((((chunk_idx * 75) + 40) / total_chunks.max(1)) as u8).min(78);
        on_progress(
            tr_pct,
            format!("Translating chunk {}/{total_chunks} ({tr_pct}%)", chunk_idx + 1),
        );
        translate_cue_batches(
            &mut chunk_cues,
            &mut translated_all,
            data_dir,
            video_id,
            &latest_cfg,
            speaker_hints.as_deref(),
        )
        .await?;
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
                "contents": [{ "parts": [{ "text": prompt }] }],
                "safetySettings": [
                    { "category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE" },
                    { "category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE" },
                    { "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE" },
                    { "category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE" }
                ]
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
        "stream": false,
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

    let target_cache = cache_path(data_dir, video_id, &config.target_lang);
    let mut bak_os = target_cache.as_os_str().to_os_string();
    bak_os.push(".bak");
    let bak_path = PathBuf::from(bak_os);

    let (source, mut cues) = match load_raw_cache(data_dir, video_id) {
        Some(raw) if !raw.partial => {
            on_progress(50, "Loaded cached transcription, resuming translation...".to_string());
            (raw.source, raw.cues)
        }
        _ => {
            on_progress(5, "Checking YouTube captions...".to_string());
            match extract_raw_cues(data_dir, bin_dir, config, url, video_id, &on_progress).await {
                Ok((src, extracted)) => {
                    let _ = fs::remove_file(&bak_path);
                    save_raw_cache(data_dir, video_id, &src, &extracted, false, 0, 0);
                    (src, extracted)
                }
                Err(err) => {
                    if !target_cache.is_file() && bak_path.is_file() {
                        let _ = fs::rename(&bak_path, &target_cache);
                    }
                    return Err(err);
                }
            }
        }
    };
    let _ = fs::remove_file(&bak_path);

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
    let remaining = &mut cues[resume_from..];
    let total_batches = remaining.len().div_ceil(TRANSLATE_BATCH_SIZE).max(1);
    let mut memory = TranslationMemory::default();

    for (b_idx, chunk) in remaining.chunks_mut(TRANSLATE_BATCH_SIZE).enumerate() {
        let done_cues = resume_from + b_idx * TRANSLATE_BATCH_SIZE;
        let pct = 55 + (((done_cues * 40) / total_cues) as u8);
        on_progress(
            pct,
            format!("Translating batch {}/{total_batches} ({pct}%)", b_idx + 1),
        );
        let prev_context = build_prev_context(&translated_all, &memory);
        translate_batch_exact(&latest_cfg, chunk, &prev_context, Some(title), &mut memory, None).await?;
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

pub fn prepare_retranslate_from_cache(
    data_dir: &Path,
    file_name: &str,
) -> Result<(String, String), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid file name".to_string());
    }
    let path = data_dir.join("subtitles").join(file_name);
    let raw = fs::read_to_string(&path).map_err(|e| format!("Subtitle cache not found: {e}"))?;
    let bundle: SubtitleBundle = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let raw_cues: Vec<SubtitleCue> = bundle
        .cues
        .iter()
        .map(|c| SubtitleCue {
            start_ms: c.start_ms,
            end_ms: c.end_ms,
            original: c.original.clone(),
            translated: String::new(),
            speaker: c.speaker.clone(),
        })
        .collect();
    if !raw_cues.is_empty() {
        save_raw_cache(data_dir, &bundle.video_id, &bundle.source, &raw_cues, false, 0, 0);
    }
    let _ = fs::remove_file(&path);
    Ok((bundle.video_id, bundle.title))
}

pub fn prepare_rewhisper_from_cache(
    data_dir: &Path,
    file_name: &str,
) -> Result<(String, String), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid file name".to_string());
    }
    let path = data_dir.join("subtitles").join(file_name);
    let raw = fs::read_to_string(&path).map_err(|e| format!("Subtitle cache not found: {e}"))?;
    let bundle: SubtitleBundle = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let mut bak_os = path.as_os_str().to_os_string();
    bak_os.push(".bak");
    let bak_path = PathBuf::from(bak_os);
    let _ = fs::remove_file(&bak_path);
    fs::rename(&path, &bak_path).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(raw_cache_path(data_dir, &bundle.video_id));
    Ok((bundle.video_id, bundle.title))
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
