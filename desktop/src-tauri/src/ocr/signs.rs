use crate::ocr::pipeline::{char_similarity, is_noise_or_watermark_token, normalize_ocr_text};
use crate::ocr::{OcrTextBox, SignCue};

#[derive(Debug, Clone)]
pub struct FrameSignCandidate {
    pub text: String,
    pub confidence: f32,
    /// Normalized [0..1] in full video frame coordinates
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fg_hex: String,
    pub bg_hex: String,
    pub solid_plate: bool,
    pub font_style: String,
}

fn rgb_luma(r: u8, g: u8, b: u8) -> f32 {
    0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32)
}

fn to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Samples foreground text color, background plate color, whether the region is a uniform plate,
/// and chooses `"serif"` vs `"sans"` style directly from the RGB buffer.
pub fn sample_sign_visual_style(
    rgb: &[u8],
    img_w: u32,
    img_h: u32,
    bx: f32,
    by: f32,
    bw: f32,
    bh: f32,
    full_h_pct: f32,
) -> (String, String, bool, String) {
    if img_w < 4 || img_h < 4 || rgb.len() < (img_w as usize * img_h as usize * 3) {
        return (
            "#FFFFFF".to_string(),
            "#111827".to_string(),
            false,
            "sans".to_string(),
        );
    }

    let x0 = ((bx * img_w as f32).floor() as i32).clamp(0, img_w as i32 - 1) as u32;
    let y0 = ((by * img_h as f32).floor() as i32).clamp(0, img_h as i32 - 1) as u32;
    let x1 = (((bx + bw) * img_w as f32).ceil() as i32).clamp((x0 + 1) as i32, img_w as i32) as u32;
    let y1 = (((by + bh) * img_h as f32).ceil() as i32).clamp((y0 + 1) as i32, img_h as i32) as u32;

    // Sample perimeter ring right around / inside edges of the box for background color
    let mut border_px: Vec<(u8, u8, u8, f32)> = Vec::new();
    let pad = 2u32;
    let rx0 = x0.saturating_sub(pad);
    let ry0 = y0.saturating_sub(pad);
    let rx1 = (x1 + pad).min(img_w);
    let ry1 = (y1 + pad).min(img_h);

    for py in ry0..ry1 {
        for px in rx0..rx1 {
            let on_edge = py <= ry0 + 1 || py + 2 >= ry1 || px <= rx0 + 1 || px + 2 >= rx1;
            if on_edge {
                let idx = ((py * img_w + px) as usize) * 3;
                let (r, g, b) = (rgb[idx], rgb[idx + 1], rgb[idx + 2]);
                border_px.push((r, g, b, rgb_luma(r, g, b)));
            }
        }
    }

    if border_px.is_empty() {
        return (
            "#FFFFFF".to_string(),
            "#111827".to_string(),
            false,
            "sans".to_string(),
        );
    }

    border_px.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
    let mid = border_px[border_px.len() / 2];
    let (mut bg_r, mut bg_g, mut bg_b, bg_luma) = mid;

    let mean_luma: f32 = border_px.iter().map(|p| p.3).sum::<f32>() / (border_px.len() as f32);
    let variance: f32 = border_px
        .iter()
        .map(|p| {
            let d = p.3 - mean_luma;
            d * d
        })
        .sum::<f32>()
        / (border_px.len() as f32);
    let bg_std = variance.sqrt();
    let solid_plate = bg_std < 24.0;

    // Sample interior pixels with strongest contrast from background luma as glyph foreground color
    let mut interior: Vec<(u8, u8, u8, f32)> = Vec::new();
    let step_x = ((x1 - x0) / 32).max(1);
    let step_y = ((y1 - y0) / 16).max(1);
    let mut py = y0;
    while py < y1 {
        let mut px = x0;
        while px < x1 {
            let idx = ((py * img_w + px) as usize) * 3;
            let (r, g, b) = (rgb[idx], rgb[idx + 1], rgb[idx + 2]);
            let diff = (rgb_luma(r, g, b) - bg_luma).abs();
            interior.push((r, g, b, diff));
            px += step_x;
        }
        py += step_y;
    }

    interior.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    let top_n = (interior.len() / 8).clamp(1, 24);
    let mut sum_r = 0u32;
    let mut sum_g = 0u32;
    let mut sum_b = 0u32;
    for &(r, g, b, _) in &interior[..top_n] {
        sum_r += r as u32;
        sum_g += g as u32;
        sum_b += b as u32;
    }
    let mut fg_r = (sum_r / top_n as u32) as u8;
    let mut fg_g = (sum_g / top_n as u32) as u8;
    let mut fg_b = (sum_b / top_n as u32) as u8;

    // Enforce crisp legibility contrast between fg and bg
    let fg_luma = rgb_luma(fg_r, fg_g, fg_b);
    if (fg_luma - bg_luma).abs() < 85.0 {
        if bg_luma > 140.0 {
            // Light plate -> dark text
            fg_r = fg_r.min(28);
            fg_g = fg_g.min(32);
            fg_b = fg_b.min(42);
            bg_r = bg_r.max(225);
            bg_g = bg_g.max(225);
            bg_b = bg_b.max(225);
        } else {
            // Dark/medium scene -> darken bg slightly & brighten fg
            bg_r = (bg_r as f32 * 0.55) as u8;
            bg_g = (bg_g as f32 * 0.55) as u8;
            bg_b = (bg_b as f32 * 0.60) as u8;
            fg_r = fg_r.max(242);
            fg_g = fg_g.max(242);
            fg_b = fg_b.max(235);
        }
    }

    let is_vertical = (bh * img_h as f32) > 1.75 * (bw * img_w as f32);
    let is_gold_or_warm = fg_r > 200 && fg_g > 165 && fg_r.saturating_sub(fg_b) > 45;
    let font_style = if full_h_pct >= 5.2 || is_vertical || is_gold_or_warm {
        "serif".to_string()
    } else {
        "sans".to_string()
    };

    (
        to_hex(fg_r, fg_g, fg_b),
        to_hex(bg_r, bg_g, bg_b),
        solid_plate,
        font_style,
    )
}

/// Groups nearby lines in a single upper-frame (e.g. Character Name + Title right below it)
/// into unified `FrameSignCandidate` blocks mapped to full-video normalized coordinates `[0..1]`.
pub fn extract_frame_signs(
    boxes: &[OcrTextBox],
    rgb: &[u8],
    img_w: u32,
    img_h: u32,
    upper_height_ratio: f32,
) -> Vec<FrameSignCandidate> {
    let mut filtered: Vec<OcrTextBox> = boxes
        .iter()
        .filter(|b| {
            let clean = normalize_ocr_text(&b.text);
            b.confidence >= 0.66
                && !clean.is_empty()
                && !is_noise_or_watermark_token(&clean)
                && b.height * upper_height_ratio >= 0.022
        })
        .cloned()
        .collect();

    if filtered.is_empty() {
        return Vec::new();
    }

    filtered.sort_by(|a, b| {
        a.y.partial_cmp(&b.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Cluster stacked lines belonging to the same nameplate
    let mut groups: Vec<Vec<OcrTextBox>> = Vec::new();
    for b in filtered {
        let mut attached = false;
        for grp in groups.iter_mut() {
            let last = grp.last().unwrap();
            let v_gap = b.y - (last.y + last.height);
            let overlap_x = (b.x + b.width).min(last.x + last.width) - b.x.max(last.x);
            let min_w = b.width.min(last.width).max(0.01);
            if v_gap >= -0.015 && v_gap <= 0.045 && (overlap_x / min_w) >= 0.35 && grp.len() < 3 {
                grp.push(b.clone());
                attached = true;
                break;
            }
        }
        if !attached {
            groups.push(vec![b]);
        }
    }

    let mut out = Vec::with_capacity(groups.len());
    for grp in groups {
        let min_x = grp.iter().map(|b| b.x).fold(1.0f32, f32::min);
        let min_y = grp.iter().map(|b| b.y).fold(1.0f32, f32::min);
        let max_x = grp.iter().map(|b| b.x + b.width).fold(0.0f32, f32::max);
        let max_y = grp.iter().map(|b| b.y + b.height).fold(0.0f32, f32::max);
        let bw = (max_x - min_x).clamp(0.02, 1.0);
        let bh = (max_y - min_y).clamp(0.02, 1.0);

        let full_y = (min_y * upper_height_ratio).clamp(0.0, 0.95);
        let full_h = (bh * upper_height_ratio).clamp(0.02, 0.50);

        // Skip tiny corner logos near extreme top-left / top-right
        let cx = min_x + bw * 0.5;
        if full_y < 0.10 && (cx < 0.16 || cx > 0.84) && bw < 0.24 {
            continue;
        }

        let joined_text = grp
            .iter()
            .map(|b| b.text.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let avg_conf = grp.iter().map(|b| b.confidence).sum::<f32>() / (grp.len() as f32);

        let (fg_hex, bg_hex, solid_plate, font_style) = sample_sign_visual_style(
            rgb,
            img_w,
            img_h,
            min_x,
            min_y,
            bw,
            bh,
            full_h * 100.0,
        );

        out.push(FrameSignCandidate {
            text: joined_text,
            confidence: avg_conf,
            x: min_x,
            y: full_y,
            w: bw,
            h: full_h,
            fg_hex,
            bg_hex,
            solid_plate,
            font_style,
        });
    }
    out
}

#[derive(Debug, Clone)]
struct SignTrack {
    start_ms: u64,
    end_ms: u64,
    best_text: String,
    best_score: f32,
    frames: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fg_hex: String,
    bg_hex: String,
    solid_plate: bool,
    font_style: String,
}

/// Merges 1fps upper-frame sign observations into stable `SignCue`s (`1.5s..15s`),
/// rejecting `<1.5s` camera-pan blur and persistent `>30%` static channel watermarks.
pub fn merge_sign_frames(
    timed_frames: &[(u64, Vec<FrameSignCandidate>)],
    step_ms: u64,
) -> Vec<SignCue> {
    if timed_frames.is_empty() {
        return Vec::new();
    }
    let total_frames = timed_frames.len();
    let mut active: Vec<SignTrack> = Vec::new();
    let mut finished: Vec<SignTrack> = Vec::new();

    for (ts_ms, cands) in timed_frames {
        let mut matched_active = vec![false; active.len()];

        for cand in cands {
            let mut best_idx = None;
            let mut best_sim = 0.0f32;
            for (idx, trk) in active.iter().enumerate() {
                if matched_active[idx] {
                    continue;
                }
                let dx = ((trk.x + trk.w * 0.5) - (cand.x + cand.w * 0.5)).abs();
                let dy = ((trk.y + trk.h * 0.5) - (cand.y + cand.h * 0.5)).abs();
                if dx > 0.09 || dy > 0.09 {
                    continue;
                }
                let sim = char_similarity(&trk.best_text, &cand.text);
                if sim >= 0.72 && sim > best_sim {
                    best_sim = sim;
                    best_idx = Some(idx);
                }
            }

            let cand_score = cand.confidence * (cand.text.chars().count().max(1) as f32).sqrt();
            if let Some(idx) = best_idx {
                matched_active[idx] = true;
                let trk = &mut active[idx];
                trk.end_ms = *ts_ms + step_ms;
                trk.frames += 1;
                if cand_score > trk.best_score {
                    trk.best_score = cand_score;
                    trk.best_text = cand.text.clone();
                    trk.x = cand.x;
                    trk.y = cand.y;
                    trk.w = cand.w;
                    trk.h = cand.h;
                    trk.fg_hex = cand.fg_hex.clone();
                    trk.bg_hex = cand.bg_hex.clone();
                    trk.solid_plate = cand.solid_plate;
                    trk.font_style = cand.font_style.clone();
                }
            } else {
                active.push(SignTrack {
                    start_ms: *ts_ms,
                    end_ms: *ts_ms + step_ms,
                    best_text: cand.text.clone(),
                    best_score: cand_score,
                    frames: 1,
                    x: cand.x,
                    y: cand.y,
                    w: cand.w,
                    h: cand.h,
                    fg_hex: cand.fg_hex.clone(),
                    bg_hex: cand.bg_hex.clone(),
                    solid_plate: cand.solid_plate,
                    font_style: cand.font_style.clone(),
                });
                matched_active.push(true);
            }
        }

        let mut next_active = Vec::new();
        for (idx, trk) in active.into_iter().enumerate() {
            if matched_active.get(idx).copied().unwrap_or(false) {
                next_active.push(trk);
            } else {
                finished.push(trk);
            }
        }
        active = next_active;
    }
    finished.extend(active);

    // Count total occurrences of normalized text across the segment to drop recurring watermarks
    let mut text_frame_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for trk in &finished {
        *text_frame_counts
            .entry(normalize_ocr_text(&trk.best_text))
            .or_insert(0) += trk.frames;
    }

    let max_allowed_frames = if total_frames >= 20 {
        ((total_frames as f32) * 0.28).ceil() as usize
    } else {
        usize::MAX
    };

    let mut signs: Vec<SignCue> = finished
        .into_iter()
        .filter(|trk| {
            let dur = trk.end_ms.saturating_sub(trk.start_ms);
            let norm = normalize_ocr_text(&trk.best_text);
            let total_occ = text_frame_counts.get(&norm).copied().unwrap_or(0);
            trk.frames >= 2
                && dur >= 1400
                && dur <= 16_000
                && total_occ <= max_allowed_frames
                && !norm.is_empty()
        })
        .map(|trk| SignCue {
            start_ms: trk.start_ms,
            end_ms: trk.end_ms,
            original: trk.best_text,
            translated: String::new(),
            x_pct: (trk.x * 100.0).clamp(1.0, 96.0),
            y_pct: (trk.y * 100.0).clamp(1.0, 88.0),
            w_pct: (trk.w * 100.0).clamp(4.0, 92.0),
            h_pct: (trk.h * 100.0).clamp(2.5, 45.0),
            fg_hex: trk.fg_hex,
            bg_hex: trk.bg_hex,
            solid_plate: trk.solid_plate,
            font_style: trk.font_style,
        })
        .collect();

    signs.sort_by_key(|s| s.start_ms);
    signs
}
