use crate::config::AppConfig;
use crate::ocr::signs::{extract_frame_signs, merge_sign_frames, FrameSignCandidate};
use crate::ocr::{create_ocr_backend, HardsubBox, LockedRoiInfo, OcrTextBox, SignCue};
use crate::tools::{build_env_path, find_binary};
use crate::translate::SubtitleCue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct OcrVideoSession {
    pub video_path: PathBuf,
    pub engine_name: String,
    pub locked_roi: LockedRoiInfo,
    /// `(start_sec, len_sec)` — Segment 0 is a fast 180s (3-min) opening chunk on long videos!
    pub segments: Vec<(f64, f64)>,
}

/// Builds segment slices `(start_sec, duration_sec)`.
/// For videos longer than 5 minutes, Segment 0 is a short 3-minute (`180s`) slice so translated
/// subtitles & character nameplates appear on YouTube within ~12–15s, followed by 10-minute slices.
pub fn build_segment_plan(dur_sec: f64) -> Vec<(f64, f64)> {
    let total = dur_sec.max(1.0);
    if total <= 300.0 {
        return vec![(0.0, total.ceil())];
    }
    let mut plan = Vec::new();
    let first = 180.0f64.min(total);
    plan.push((0.0, first));
    let mut cur = first;
    let std_seg = 600.0f64;
    while cur < total - 0.5 {
        let rem = total - cur;
        // Merge tiny trailing remainder (<90s) into the last segment
        let len = if rem <= std_seg + 90.0 { rem.ceil() } else { std_seg };
        plan.push((cur, len));
        cur += len;
    }
    plan
}

pub fn normalize_ocr_text(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '·' | '•' | '-' | '_' | '~' | '|'))
        .collect::<String>()
        .to_lowercase()
}

pub fn is_noise_or_watermark_token(norm: &str) -> bool {
    if norm.is_empty() {
        return true;
    }
    if norm
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, ':' | '/' | '.' | '%' | '+'))
    {
        return true;
    }
    let lower = norm.to_lowercase();
    if lower.starts_with('@')
        || lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("www.")
        || lower.contains("抖音")
        || lower.contains("快手")
        || lower.contains("bilibili")
        || lower.contains("youtube")
        || lower.contains("subscribe")
    {
        return true;
    }
    false
}

/// Normalized character similarity `[0.0..1.0]` using Levenshtein edit distance on `char`s.
pub fn char_similarity(a: &str, b: &str) -> f32 {
    let na: Vec<char> = normalize_ocr_text(a).chars().collect();
    let nb: Vec<char> = normalize_ocr_text(b).chars().collect();
    if na.is_empty() && nb.is_empty() {
        return 1.0;
    }
    if na.is_empty() || nb.is_empty() {
        return 0.0;
    }
    if na == nb {
        return 1.0;
    }
    let max_len = na.len().max(nb.len());
    let mut prev: Vec<usize> = (0..=nb.len()).collect();
    let mut curr = vec![0usize; nb.len() + 1];
    for (i, &ca) in na.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in nb.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    let dist = prev[nb.len()];
    (1.0 - (dist as f32 / max_len as f32)).clamp(0.0, 1.0)
}

/// Computes Mean Absolute Difference (`0.0..255.0`) across the middle 72% horizontal band of two RGB buffers.
pub fn compute_center_mad_rgb(a: &[u8], b: &[u8], width: u32, height: u32) -> f32 {
    if a.len() != b.len() || width < 8 || height < 4 {
        return 255.0;
    }
    let x_start = (width as usize) * 14 / 100;
    let x_end = (width as usize) * 86 / 100;
    let step_y = ((height as usize) / 24).max(1);
    let step_x = ((x_end.saturating_sub(x_start)) / 64).max(1);

    let mut diff_sum = 0u64;
    let mut samples = 0u64;
    let mut y = 0usize;
    while y < height as usize {
        let mut x = x_start;
        while x < x_end {
            let idx = (y * (width as usize) + x) * 3;
            if idx + 2 < a.len() {
                let la =
                    (a[idx] as i32 * 77 + a[idx + 1] as i32 * 150 + a[idx + 2] as i32 * 29) >> 8;
                let lb =
                    (b[idx] as i32 * 77 + b[idx + 1] as i32 * 150 + b[idx + 2] as i32 * 29) >> 8;
                diff_sum += (la - lb).unsigned_abs() as u64;
                samples += 1;
            }
            x += step_x;
        }
        y += step_y;
    }
    if samples == 0 {
        255.0
    } else {
        (diff_sum as f32) / (samples as f32)
    }
}

/// Parses video `(duration_ms, width, height)` directly from `ffmpeg -hide_banner -i <video>` stderr
/// so we never require a separate `ffprobe` binary on Windows/Linux/macOS.
pub fn parse_ffmpeg_probe_stderr(stderr: &str) -> (u64, u32, u32) {
    let mut duration_ms = 0u64;
    let mut width = 0u32;
    let mut height = 0u32;

    for line in stderr.lines() {
        let trimmed = line.trim();
        if duration_ms == 0 {
            if let Some(idx) = trimmed.find("Duration:") {
                let rest = trimmed[idx + 9..].trim();
                let time_part = rest.split(',').next().unwrap_or("").trim();
                let parts: Vec<&str> = time_part.split(':').collect();
                if parts.len() == 3 {
                    let h: f64 = parts[0].parse().unwrap_or(0.0);
                    let m: f64 = parts[1].parse().unwrap_or(0.0);
                    let s: f64 = parts[2].parse().unwrap_or(0.0);
                    duration_ms = ((h * 3600.0 + m * 60.0 + s) * 1000.0).round() as u64;
                }
            }
        }
        if width == 0 && trimmed.contains("Video:") {
            for token in trimmed.split(|c: char| c == ',' || c.is_whitespace()) {
                if let Some((w_str, h_str)) = token.split_once('x') {
                    let w_clean = w_str.trim_matches(|c: char| !c.is_ascii_digit());
                    let h_clean = h_str
                        .split(|c: char| !c.is_ascii_digit())
                        .next()
                        .unwrap_or("");
                    if let (Ok(w), Ok(h)) = (w_clean.parse::<u32>(), h_clean.parse::<u32>()) {
                        if (120..=7680).contains(&w) && (120..=7680).contains(&h) {
                            width = w;
                            height = h;
                            break;
                        }
                    }
                }
            }
        }
    }

    if width == 0 || height == 0 {
        (duration_ms, 640, 360)
    } else {
        (duration_ms, width, height)
    }
}

fn default_probe_band(region_cfg: &str, width: u32, height: u32) -> (f32, f32) {
    match region_cfg {
        "bottom" => (0.72, 0.26),
        "shorts" => (0.56, 0.37),
        _ => {
            if height > width {
                (0.55, 0.38)
            } else {
                (0.66, 0.32)
            }
        }
    }
}

/// Converts per-frame ROI OCR boxes (`4 fps` = `250ms` per frame) into deduplicated `SubtitleCue`s,
/// filtering static watermarks and merging fade-in/fade-out partial reads.
pub fn merge_roi_frames_into_cues(
    timed_boxes: &[(u64, Vec<OcrTextBox>)],
    frame_step_ms: u64,
) -> Vec<SubtitleCue> {
    if timed_boxes.is_empty() {
        return Vec::new();
    }

    let mut frame_lines: Vec<(u64, String, f32)> = Vec::with_capacity(timed_boxes.len());
    let mut norm_counts: HashMap<String, usize> = HashMap::new();
    let mut non_empty_frames = 0usize;

    for (ts_ms, boxes) in timed_boxes {
        let mut valid: Vec<&OcrTextBox> = boxes
            .iter()
            .filter(|b| {
                let cx = b.center_x();
                let norm = normalize_ocr_text(&b.text);
                b.confidence >= 0.56
                    && (0.16..=0.84).contains(&cx)
                    && !norm.is_empty()
                    && !is_noise_or_watermark_token(&norm)
            })
            .collect();

        if valid.is_empty() {
            frame_lines.push((*ts_ms, String::new(), 0.0));
            continue;
        }

        valid.sort_by(|a, b| {
            a.y.partial_cmp(&b.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });
        valid.truncate(2);

        let text = valid
            .iter()
            .map(|b| b.text.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let avg_conf = valid.iter().map(|b| b.confidence).sum::<f32>() / (valid.len() as f32);
        let norm = normalize_ocr_text(&text);
        if !norm.is_empty() {
            non_empty_frames += 1;
            *norm_counts.entry(norm).or_insert(0) += 1;
        }
        frame_lines.push((*ts_ms, text, avg_conf));
    }

    let watermark_limit = if non_empty_frames >= 24 {
        ((non_empty_frames as f32) * 0.35).ceil() as usize
    } else {
        usize::MAX
    };

    for (_, text, conf) in frame_lines.iter_mut() {
        if !text.is_empty() {
            let norm = normalize_ocr_text(text);
            if norm_counts.get(&norm).copied().unwrap_or(0) > watermark_limit {
                text.clear();
                *conf = 0.0;
            }
        }
    }

    if frame_lines.len() >= 3 {
        for i in 1..(frame_lines.len() - 1) {
            if frame_lines[i].1.is_empty()
                && !frame_lines[i - 1].1.is_empty()
                && !frame_lines[i + 1].1.is_empty()
                && char_similarity(&frame_lines[i - 1].1, &frame_lines[i + 1].1) >= 0.85
            {
                frame_lines[i].1 = frame_lines[i - 1].1.clone();
                frame_lines[i].2 = frame_lines[i - 1].2;
            }
        }
    }

    struct ActiveCue {
        start_ms: u64,
        end_ms: u64,
        best_text: String,
        best_score: f32,
        best_conf: f32,
        frame_count: usize,
    }

    let mut cues: Vec<SubtitleCue> = Vec::new();
    let mut cur: Option<ActiveCue> = None;

    for (ts_ms, text, conf) in frame_lines {
        if text.is_empty() {
            if let Some(fin) = cur.take() {
                if fin.frame_count >= 2 || fin.best_conf >= 0.74 {
                    cues.push(SubtitleCue {
                        start_ms: fin.start_ms,
                        end_ms: fin.end_ms,
                        original: fin.best_text,
                        translated: String::new(),
                        speaker: None,
                    });
                }
            }
            continue;
        }

        let cand_score = conf * (text.chars().count().max(1) as f32).sqrt();
        if let Some(ref mut active) = cur {
            let sim = char_similarity(&active.best_text, &text);
            let na = normalize_ocr_text(&active.best_text);
            let nb = normalize_ocr_text(&text);
            let min_chars = na.chars().count().min(nb.chars().count());
            let fade_substr =
                (na.contains(&nb) || nb.contains(&na)) && min_chars >= 3 && active.frame_count <= 3;

            if sim >= 0.78 || fade_substr {
                active.end_ms = ts_ms + frame_step_ms;
                active.frame_count += 1;
                if cand_score > active.best_score {
                    active.best_score = cand_score;
                    active.best_text = text;
                    active.best_conf = conf;
                }
                continue;
            }

            let finished = cur.take().unwrap();
            if finished.frame_count >= 2 || finished.best_conf >= 0.74 {
                cues.push(SubtitleCue {
                    start_ms: finished.start_ms,
                    end_ms: ts_ms,
                    original: finished.best_text,
                    translated: String::new(),
                    speaker: None,
                });
            }
        }

        cur = Some(ActiveCue {
            start_ms: ts_ms,
            end_ms: ts_ms + frame_step_ms,
            best_text: text,
            best_score: cand_score,
            best_conf: conf,
            frame_count: 1,
        });
    }

    if let Some(fin) = cur {
        if fin.frame_count >= 2 || fin.best_conf >= 0.74 {
            cues.push(SubtitleCue {
                start_ms: fin.start_ms,
                end_ms: fin.end_ms,
                original: fin.best_text,
                translated: String::new(),
                speaker: None,
            });
        }
    }

    cues
}

async fn download_lowres_video_for_ocr<F>(
    ytdlp: &Path,
    ffmpeg: &Path,
    bin_dir: &Path,
    config: &AppConfig,
    url: &str,
    video_id: &str,
    tmp_dir: &Path,
    on_progress: &F,
) -> Result<PathBuf, String>
where
    F: Fn(u8, String),
{
    let video_path = tmp_dir.join("ocr_video.mp4");
    if video_path.is_file() {
        return Ok(video_path);
    }

    // Strictly prefer H.264 (avc1) 360p/480p over AV1 (av01) so ffmpeg decodes 3-4x faster
    // with minimal CPU and clean container timestamps!
    let mut args = vec![
        "--newline".to_string(),
        "--no-playlist".to_string(),
        "-f".to_string(),
        "bv*[vcodec^=avc1][height<=360]/bv*[vcodec^=avc1][height<=480]/b[vcodec^=avc1][height<=480]/bv*[vcodec^=avc1][height<=720]/bv*[height<=360]/bv*[height<=480]/b[height<=480]/b".to_string(),
        "-o".to_string(),
        video_path.to_string_lossy().to_string(),
    ];
    if !config.cookies_browser.is_empty() {
        args.push("--cookies-from-browser".to_string());
        args.push(config.cookies_browser.clone());
    }
    args.push(url.to_string());

    on_progress(6, "Downloading lightweight H.264 video stream for OCR...".to_string());
    let mut child = Command::new(ytdlp)
        .args(&args)
        .env("PATH", build_env_path(bin_dir))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp for OCR video: {e}"))?;

    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(idx) = line.find("[download]") {
                if let Some(pct_idx) = line[idx + 10..].find('%') {
                    if let Some(num) = line[idx + 10..idx + 10 + pct_idx]
                        .trim()
                        .split_whitespace()
                        .last()
                    {
                        if let Ok(val) = num.parse::<f64>() {
                            let p = 6 + ((val.clamp(0.0, 100.0) * 0.08) as u8);
                            on_progress(
                                p,
                                format!("Downloading OCR video stream {}%", val.round() as u8),
                            );
                        }
                    }
                }
            }
        }
    }

    let out = child
        .wait_with_output()
        .await
        .map_err(|e| format!("yt-dlp OCR video failed: {e}"))?;

    if out.status.success() && video_path.is_file() {
        return Ok(video_path);
    }

    let stream_url = format!("http://127.0.0.1:{}/stream/{}", config.port, video_id);
    on_progress(
        12,
        "Trying Cloud video stream for Hardsub OCR...".to_string(),
    );
    let ff_out = Command::new(ffmpeg)
        .args([
            "-y",
            "-i",
            &stream_url,
            "-an",
            "-vf",
            "scale='min(640,iw)':-2",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "30",
            &video_path.to_string_lossy(),
        ])
        .output()
        .await;

    if let Ok(o) = ff_out {
        if o.status.success() && video_path.is_file() {
            return Ok(video_path);
        }
    }

    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(stderr
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("Failed to download video stream for Hardsub OCR")
        .to_string())
}

fn list_sorted_jpegs(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix) && n.ends_with(".jpg"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files
}

/// Step 1: Downloads lightweight H.264 video stream and runs Fast 12-frame Hardsub Probe.
/// Returns `Ok(None)` when `ocr_mode == "auto"` and no Hardsub is present so caller falls back to Whisper.
pub async fn prepare_video_ocr_session<F>(
    data_dir: &Path,
    bin_dir: &Path,
    config: &AppConfig,
    url: &str,
    video_id: &str,
    tmp_dir: &Path,
    force_ocr: bool,
    on_progress: &F,
) -> Result<Option<OcrVideoSession>, String>
where
    F: Fn(u8, String),
{
    let ytdlp =
        find_binary("yt-dlp", bin_dir).ok_or_else(|| "yt-dlp is not installed".to_string())?;
    let ffmpeg =
        find_binary("ffmpeg", bin_dir).ok_or_else(|| "ffmpeg is not installed".to_string())?;

    let video_path = download_lowres_video_for_ocr(
        &ytdlp,
        &ffmpeg,
        bin_dir,
        config,
        url,
        video_id,
        tmp_dir,
        on_progress,
    )
    .await?;

    let probe_meta = Command::new(&ffmpeg)
        .args(["-hide_banner", "-i", &video_path.to_string_lossy()])
        .output()
        .await
        .map_err(|e| format!("ffmpeg probe failed: {e}"))?;
    let meta_stderr = String::from_utf8_lossy(&probe_meta.stderr);
    let (duration_ms, vid_w, vid_h) = parse_ffmpeg_probe_stderr(&meta_stderr);
    let dur_sec = ((duration_ms as f64) / 1000.0).max(8.0);

    on_progress(15, "Probing video frames for Hardsub...".to_string());
    let probe_dir = tmp_dir.join("ocr_probe");
    let _ = fs::remove_dir_all(&probe_dir);
    fs::create_dir_all(&probe_dir).map_err(|e| e.to_string())?;

    let (band_y, band_h) = default_probe_band(&config.ocr_region, vid_w, vid_h);
    let start_sec = (dur_sec * 0.08).min(5.0);
    let span_sec = (dur_sec * 0.82).max(4.0);
    let step_sec = (span_sec / 12.0).max(0.8);
    let probe_vf = format!(
        "fps=1/{step_sec:.3},crop=iw:ih*{band_h:.4}:0:ih*{band_y:.4},scale='min(640,iw)':-2"
    );

    let _ = Command::new(&ffmpeg)
        .args([
            "-y",
            "-ss",
            &format!("{start_sec:.2}"),
            "-i",
            &video_path.to_string_lossy(),
            "-an",
            "-vf",
            &probe_vf,
            "-frames:v",
            "13",
            "-q:v",
            "3",
            &probe_dir.join("probe_%03d.jpg").to_string_lossy(),
        ])
        .output()
        .await;

    let probe_files = list_sorted_jpegs(&probe_dir, "probe_");
    let lang_hint = config.whisper_language.clone();
    let d_dir = data_dir.to_path_buf();

    let probe_res =
        tokio::task::spawn_blocking(move || -> Result<(Option<LockedRoiInfo>, String), String> {
            let mut backend = create_ocr_backend(&d_dir, 0.06)?;
            let engine_name = backend.engine_name().to_string();
            let mut hit_ys: Vec<(f32, f32)> = Vec::new();
            let mut text_counts: HashMap<String, usize> = HashMap::new();
            let mut valid_frames = 0usize;

            for p in &probe_files {
                let Ok(img) = image::open(p) else { continue };
                let rgb = img.to_rgb8();
                let (w, h) = rgb.dimensions();
                let boxes = backend.recognize_image(rgb.as_raw(), w, h, &lang_hint)?;
                let mut frame_texts = Vec::new();
                for b in boxes {
                    let norm = normalize_ocr_text(&b.text);
                    if b.confidence >= 0.60
                        && (0.18..=0.82).contains(&b.center_x())
                        && !norm.is_empty()
                        && !is_noise_or_watermark_token(&norm)
                    {
                        let full_cy = band_y + b.center_y() * band_h;
                        let full_h = b.height * band_h;
                        hit_ys.push((full_cy, full_h));
                        frame_texts.push(norm);
                    }
                }
                if !frame_texts.is_empty() {
                    valid_frames += 1;
                    let joined = frame_texts.join("");
                    *text_counts.entry(joined).or_insert(0) += 1;
                }
            }

            let distinct_texts = text_counts.len();
            let min_hits = if dur_sec < 20.0 { 1 } else { 2 };
            if valid_frames < min_hits || (valid_frames >= 3 && distinct_texts <= 1) {
                return Ok((None, engine_name));
            }

            hit_ys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let (med_cy, med_h) = hit_ys[hit_ys.len() / 2];
            let max_h = hit_ys
                .iter()
                .map(|x| x.1)
                .fold(med_h, f32::max)
                .clamp(0.035, 0.16);

            let y_top = (med_cy - max_h * 1.9).clamp(0.48, 0.86);
            let y_bot = (med_cy + max_h * 1.9).clamp(y_top + 0.14, 0.99);
            let roi_h = y_bot - y_top;

            let bottom_pct = ((1.0 - (med_cy + med_h * 0.65)) * 100.0).clamp(2.0, 40.0);
            let height_pct = (max_h * 145.0).clamp(6.5, 22.0);

            Ok((
                Some(LockedRoiInfo {
                    y_top,
                    height: roi_h,
                    hardsub_box: HardsubBox {
                        bottom_pct,
                        height_pct,
                    },
                }),
                engine_name,
            ))
        })
        .await
        .map_err(|e| format!("OCR probe task error: {e}"))??;

    let _ = fs::remove_dir_all(&probe_dir);

    let (locked_roi, engine_name) = match probe_res {
        (Some(roi), eng) => (roi, eng),
        (None, eng) => {
            if !force_ocr && config.ocr_mode != "ocr_only" {
                let _ = fs::remove_file(&video_path);
                on_progress(
                    18,
                    "No Hardsub detected on video — falling back to Whisper STT...".to_string(),
                );
                return Ok(None);
            }
            (
                LockedRoiInfo {
                    y_top: band_y,
                    height: band_h,
                    hardsub_box: HardsubBox {
                        bottom_pct: ((1.0 - (band_y + band_h * 0.75)) * 100.0).clamp(4.0, 30.0),
                        height_pct: 11.0,
                    },
                },
                eng,
            )
        }
    };

    Ok(Some(OcrVideoSession {
        video_path,
        engine_name,
        locked_roi,
        segments: build_segment_plan(dur_sec),
    }))
}

/// Scans a single `(seg_start_sec, seg_len_sec)` slice using a SINGLE-PASS `ffmpeg` decode
/// (extracting both `4 fps` ROI and `1 fps` Scene Signs simultaneously when enabled) + Pixel-Diff Gate.
pub async fn scan_video_ocr_segment(
    data_dir: &Path,
    ffmpeg: &Path,
    config: &AppConfig,
    session: &OcrVideoSession,
    tmp_dir: &Path,
    seg_idx: usize,
    seg_start_sec: f64,
    seg_len_sec: f64,
) -> Result<(Vec<SubtitleCue>, Vec<SignCue>), String> {
    let seg_offset_ms = (seg_start_sec * 1000.0).round() as u64;
    let seg_dir = tmp_dir.join(format!("ocr_seg_{seg_idx:03}"));
    let _ = fs::remove_dir_all(&seg_dir);
    fs::create_dir_all(&seg_dir).map_err(|e| e.to_string())?;

    let scan_signs = config.ocr_translate_signs && session.locked_roi.y_top >= 0.25;
    let roi_out_pattern = seg_dir.join("roi_%05d.jpg").to_string_lossy().to_string();

    let ff_status = if scan_signs {
        let upper_h = session.locked_roi.y_top.clamp(0.25, 0.85);
        let sign_out_pattern = seg_dir.join("sign_%04d.jpg").to_string_lossy().to_string();
        let filter_complex = format!(
            "[0:v]split=2[vroi][vsign];[vroi]fps=4,crop=iw:ih*{:.4}:0:ih*{:.4},scale='min(640,iw)':-2[outroi];[vsign]fps=1,crop=iw:ih*{upper_h:.4}:0:0,scale='min(640,iw)':-2[outsign]",
            session.locked_roi.height, session.locked_roi.y_top
        );
        Command::new(ffmpeg)
            .args([
                "-y",
                "-ss",
                &format!("{seg_start_sec:.2}"),
                "-t",
                &format!("{seg_len_sec:.2}"),
                "-i",
                &session.video_path.to_string_lossy(),
                "-an",
                "-filter_complex",
                &filter_complex,
                "-map",
                "[outroi]",
                "-q:v",
                "3",
                &roi_out_pattern,
                "-map",
                "[outsign]",
                "-q:v",
                "4",
                &sign_out_pattern,
            ])
            .output()
            .await
            .map_err(|e| format!("ffmpeg 1-pass extract failed: {e}"))?
            .status
    } else {
        let roi_vf = format!(
            "fps=4,crop=iw:ih*{:.4}:0:ih*{:.4},scale='min(640,iw)':-2",
            session.locked_roi.height, session.locked_roi.y_top
        );
        Command::new(ffmpeg)
            .args([
                "-y",
                "-ss",
                &format!("{seg_start_sec:.2}"),
                "-t",
                &format!("{seg_len_sec:.2}"),
                "-i",
                &session.video_path.to_string_lossy(),
                "-an",
                "-vf",
                &roi_vf,
                "-q:v",
                "3",
                &roi_out_pattern,
            ])
            .output()
            .await
            .map_err(|e| format!("ffmpeg ROI extract failed: {e}"))?
            .status
    };

    if !ff_status.success() {
        let _ = fs::remove_dir_all(&seg_dir);
        return Err("Failed to extract video frames with ffmpeg".to_string());
    }

    let roi_files = list_sorted_jpegs(&seg_dir, "roi_");
    let sign_files = if scan_signs {
        list_sorted_jpegs(&seg_dir, "sign_")
    } else {
        Vec::new()
    };

    let d_dir = data_dir.to_path_buf();
    let lang_hint = config.whisper_language.clone();
    let upper_ratio = session.locked_roi.y_top;

    let (seg_roi_frames, seg_sign_frames) = tokio::task::spawn_blocking(
        move || -> Result<(Vec<(u64, Vec<OcrTextBox>)>, Vec<(u64, Vec<FrameSignCandidate>)>), String> {
            let mut roi_backend = create_ocr_backend(&d_dir, 0.07)?;
            let mut prev_rgb: Option<(u32, u32, Vec<u8>)> = None;
            let mut prev_boxes: Vec<OcrTextBox> = Vec::new();
            let mut roi_res = Vec::with_capacity(roi_files.len());

            for (f_idx, path) in roi_files.iter().enumerate() {
                let ts_ms = seg_offset_ms + (f_idx as u64) * 250;
                let Ok(img) = image::open(path) else {
                    roi_res.push((ts_ms, Vec::new()));
                    continue;
                };
                let rgb = img.to_rgb8();
                let (w, h) = rgb.dimensions();
                let raw = rgb.into_raw();

                let unchanged = prev_rgb
                    .as_ref()
                    .map(|(pw, ph, pr)| {
                        *pw == w && *ph == h && compute_center_mad_rgb(pr, &raw, w, h) < 3.2
                    })
                    .unwrap_or(false);

                if unchanged {
                    roi_res.push((ts_ms, prev_boxes.clone()));
                } else {
                    let boxes = roi_backend.recognize_image(&raw, w, h, &lang_hint)?;
                    prev_boxes = boxes.clone();
                    prev_rgb = Some((w, h, raw));
                    roi_res.push((ts_ms, boxes));
                }
            }

            let mut sign_res = Vec::with_capacity(sign_files.len());
            if !sign_files.is_empty() {
                let mut sign_backend = create_ocr_backend(&d_dir, 0.022)?;
                let mut prev_sign_rgb: Option<(u32, u32, Vec<u8>)> = None;
                let mut prev_sign_cands: Vec<FrameSignCandidate> = Vec::new();

                for (s_idx, path) in sign_files.iter().enumerate() {
                    let ts_ms = seg_offset_ms + (s_idx as u64) * 1000;
                    let Ok(img) = image::open(path) else {
                        sign_res.push((ts_ms, Vec::new()));
                        continue;
                    };
                    let rgb = img.to_rgb8();
                    let (w, h) = rgb.dimensions();
                    let raw = rgb.into_raw();

                    let unchanged = prev_sign_rgb
                        .as_ref()
                        .map(|(pw, ph, pr)| {
                            *pw == w && *ph == h && compute_center_mad_rgb(pr, &raw, w, h) < 3.8
                        })
                        .unwrap_or(false);

                    if unchanged {
                        sign_res.push((ts_ms, prev_sign_cands.clone()));
                    } else {
                        let boxes = sign_backend.recognize_image(&raw, w, h, &lang_hint)?;
                        let cands = extract_frame_signs(&boxes, &raw, w, h, upper_ratio);
                        prev_sign_cands = cands.clone();
                        prev_sign_rgb = Some((w, h, raw));
                        sign_res.push((ts_ms, cands));
                    }
                }
            }

            Ok((roi_res, sign_res))
        },
    )
    .await
    .map_err(|e| format!("OCR segment worker failed: {e}"))??;

    let _ = fs::remove_dir_all(&seg_dir);

    let cues = merge_roi_frames_into_cues(&seg_roi_frames, 250);
    let signs = if scan_signs {
        merge_sign_frames(&seg_sign_frames, 1000)
    } else {
        Vec::new()
    };

    Ok((cues, signs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_box(text: &str, conf: f32, cx: f32) -> OcrTextBox {
        OcrTextBox {
            text: text.to_string(),
            confidence: conf,
            x: (cx - 0.2).max(0.0),
            y: 0.3,
            width: 0.4,
            height: 0.25,
        }
    }

    #[test]
    fn test_segment_plan_fast_first_3min_for_long_movies() {
        let short = build_segment_plan(240.0);
        assert_eq!(short, vec![(0.0, 240.0)]);

        // 2h11m50s = 7910.3s -> Segment 0 is 180s (3 min), followed by 600s chunks
        let long = build_segment_plan(7910.3);
        assert_eq!(long[0], (0.0, 180.0));
        assert_eq!(long[1], (180.0, 600.0));
        let last = long.last().unwrap();
        assert!((last.0 + last.1) >= 7910.3);
    }

    #[test]
    fn test_parse_ffmpeg_probe_stderr_landscape_and_shorts() {
        let stderr = "  Duration: 00:12:34.50, start: 0.000000, bitrate: 312 kb/s\n  Stream #0:0(und): Video: h264 (Main), yuv420p(tv), 540x960 [SAR 1:1 DAR 9:16], 30 fps";
        let (dur, w, h) = parse_ffmpeg_probe_stderr(stderr);
        assert_eq!(dur, 754_500);
        assert_eq!(w, 540);
        assert_eq!(h, 960);
    }

    #[test]
    fn test_merge_roi_frames_merges_fades_and_filters_watermarks() {
        let mut frames: Vec<(u64, Vec<OcrTextBox>)> = Vec::new();
        frames.push((0, vec![mk_box("傅总，夫人已经签", 0.68, 0.50)]));
        for t in [250, 500, 750] {
            frames.push((t, vec![mk_box("傅总，夫人已经签字了！", 0.96, 0.50)]));
        }
        frames.push((1000, vec![]));
        frames.push((1250, vec![mk_box("傅总，夫人已经签字了！", 0.95, 0.50)]));
        frames.push((1500, vec![mk_box("你说什么？！", 0.94, 0.50)]));
        frames.push((1750, vec![mk_box("你说什么？！", 0.97, 0.50)]));

        let cues = merge_roi_frames_into_cues(&frames, 250);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].original, "傅总，夫人已经签字了！");
        assert_eq!(cues[0].start_ms, 0);
        assert_eq!(cues[0].end_ms, 1500);
        assert_eq!(cues[1].original, "你说什么？！");
        assert_eq!(cues[1].start_ms, 1500);
        assert_eq!(cues[1].end_ms, 2000);
    }

    #[test]
    fn test_pixel_diff_mad_identifies_static_vs_changed_frames() {
        let w = 64u32;
        let h = 16u32;
        let a = vec![20u8; (w * h * 3) as usize];
        let mut b = a.clone();
        assert!(compute_center_mad_rgb(&a, &b, w, h) < 0.01);
        for y in 4..12 {
            for x in 16..48 {
                let idx = ((y * w + x) * 3) as usize;
                b[idx] = 245;
                b[idx + 1] = 245;
                b[idx + 2] = 245;
            }
        }
        assert!(compute_center_mad_rgb(&a, &b, w, h) > 40.0);
    }
}
