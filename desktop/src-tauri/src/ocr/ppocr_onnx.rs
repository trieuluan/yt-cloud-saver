use crate::ocr::{OcrBackend, OcrTextBox};
use image::imageops::FilterType;
use image::{DynamicImage, RgbImage};
use ort::session::Session;
use ort::value::Tensor;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

pub const PPOCR_DET_URL: &str =
    "https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_det_infer.onnx";
pub const PPOCR_REC_URL: &str =
    "https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_rec_infer.onnx";
pub const PPOCR_KEYS_URL: &str =
    "https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/ppocr/utils/ppocr_keys_v1.txt";

pub fn ocr_models_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join("ocr")
}

pub fn ppocr_det_path(data_dir: &Path) -> PathBuf {
    ocr_models_dir(data_dir).join("ch_PP-OCRv4_det_infer.onnx")
}

pub fn ppocr_rec_path(data_dir: &Path) -> PathBuf {
    ocr_models_dir(data_dir).join("ch_PP-OCRv4_rec_infer.onnx")
}

pub fn ppocr_keys_path(data_dir: &Path) -> PathBuf {
    ocr_models_dir(data_dir).join("ppocr_keys_v1.txt")
}

pub fn is_ppocr_installed(data_dir: &Path) -> bool {
    ppocr_det_path(data_dir).is_file()
        && ppocr_rec_path(data_dir).is_file()
        && ppocr_keys_path(data_dir).is_file()
}

pub async fn ensure_ppocr_models(data_dir: &Path) -> Result<(), String> {
    let dir = ocr_models_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let det = ppocr_det_path(data_dir);
    if !det.is_file() {
        crate::tools::download_to_file(PPOCR_DET_URL, &det).await?;
    }
    let rec = ppocr_rec_path(data_dir);
    if !rec.is_file() {
        crate::tools::download_to_file(PPOCR_REC_URL, &rec).await?;
    }
    let keys = ppocr_keys_path(data_dir);
    if !keys.is_file() {
        crate::tools::download_to_file(PPOCR_KEYS_URL, &keys).await?;
    }
    Ok(())
}

pub fn remove_ppocr_models(data_dir: &Path) -> Result<(), String> {
    let dir = ocr_models_dir(data_dir);
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub struct PpOcrOnnxBackend {
    det_session: Session,
    rec_session: Session,
    dict: Vec<String>,
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
impl PpOcrOnnxBackend {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        if !is_ppocr_installed(data_dir) {
            return Err(
                "PP-OCRv4 ONNX models (~15.2 MB) are not downloaded. Open AI & Translate to install."
                    .to_string(),
            );
        }
        let env = crate::yamnet::shared_ort_env()?;
        let det_session = Session::builder(env)
            .map_err(|e| format!("ORT builder error: {e}"))?
            .commit_from_file(ppocr_det_path(data_dir))
            .map_err(|e| format!("Failed to load PP-OCRv4 det model: {e}"))?;
        let rec_session = Session::builder(env)
            .map_err(|e| format!("ORT builder error: {e}"))?
            .commit_from_file(ppocr_rec_path(data_dir))
            .map_err(|e| format!("Failed to load PP-OCRv4 rec model: {e}"))?;

        let raw_keys = fs::read_to_string(ppocr_keys_path(data_dir))
            .map_err(|e| format!("Failed to read ppocr_keys_v1.txt: {e}"))?;
        let mut dict: Vec<String> = raw_keys
            .lines()
            .map(|l| l.trim_end_matches(['\r', '\n']).to_string())
            .collect();
        // PP-OCRv4 appends space character at the end of ppocr_keys_v1
        dict.push(" ".to_string());

        Ok(Self {
            det_session,
            rec_session,
            dict,
        })
    }

    fn detect_boxes(&mut self, img: &RgbImage) -> Result<Vec<(u32, u32, u32, u32)>, String> {
        let (orig_w, orig_h) = img.dimensions();
        if orig_w < 8 || orig_h < 8 {
            return Ok(Vec::new());
        }

        let max_side = 960.0f32;
        let scale = if (orig_w.max(orig_h) as f32) > max_side {
            max_side / (orig_w.max(orig_h) as f32)
        } else {
            1.0
        };
        let resized_w = (((orig_w as f32 * scale).round() as u32).div_ceil(32) * 32).max(32);
        let resized_h = (((orig_h as f32 * scale).round() as u32).div_ceil(32) * 32).max(32);

        let resized = if resized_w == orig_w && resized_h == orig_h {
            img.clone()
        } else {
            image::imageops::resize(img, resized_w, resized_h, FilterType::Triangle)
        };

        let hw = (resized_w as usize) * (resized_h as usize);
        let mut chw = vec![0.0f32; 3 * hw];
        // PaddleOCR DBNet expects BGR normalized with ImageNet mean/std
        const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
        const STD: [f32; 3] = [0.229, 0.224, 0.225];
        let raw = resized.as_raw();
        for idx in 0..hw {
            let r = raw[idx * 3] as f32 / 255.0;
            let g = raw[idx * 3 + 1] as f32 / 255.0;
            let b = raw[idx * 3 + 2] as f32 / 255.0;
            chw[idx] = (b - MEAN[0]) / STD[0];
            chw[hw + idx] = (g - MEAN[1]) / STD[1];
            chw[2 * hw + idx] = (r - MEAN[2]) / STD[2];
        }

        let input_tensor = Tensor::from_array((
            [1usize, 3, resized_h as usize, resized_w as usize],
            chw.into_boxed_slice(),
        ))
        .map_err(|e| format!("PP-OCR det tensor error: {e}"))?;

        let outputs = self
            .det_session
            .run(ort::inputs![input_tensor])
            .map_err(|e| format!("PP-OCR det run error: {e}"))?;

        let (shape, prob) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("PP-OCR det output error: {e}"))?;
        if shape.len() != 4 || prob.len() < hw {
            return Ok(Vec::new());
        }

        Ok(extract_dbnet_boxes(
            &prob[..hw],
            resized_w,
            resized_h,
            orig_w,
            orig_h,
        ))
    }

    fn recognize_crop(&mut self, crop: &RgbImage) -> Result<(String, f32), String> {
        let (cw, ch) = crop.dimensions();
        if cw < 4 || ch < 4 {
            return Ok((String::new(), 0.0));
        }

        // Rotate vertical CJK column crops (height > 2.0 * width) 90 deg CCW
        let oriented = if ch > cw.saturating_mul(2) {
            DynamicImage::ImageRgb8(crop.clone()).rotate270().to_rgb8()
        } else {
            crop.clone()
        };
        let (ow, oh) = oriented.dimensions();

        let target_h = 48u32;
        let target_w = (((ow as f32) * (target_h as f32) / (oh.max(1) as f32)).ceil() as u32)
            .clamp(32, 1280);
        let resized = image::imageops::resize(&oriented, target_w, target_h, FilterType::Triangle);

        let hw = (target_w as usize) * (target_h as usize);
        let mut chw = vec![0.0f32; 3 * hw];
        let raw = resized.as_raw();
        for idx in 0..hw {
            let r = raw[idx * 3] as f32 / 255.0;
            let g = raw[idx * 3 + 1] as f32 / 255.0;
            let b = raw[idx * 3 + 2] as f32 / 255.0;
            chw[idx] = (b - 0.5) / 0.5;
            chw[hw + idx] = (g - 0.5) / 0.5;
            chw[2 * hw + idx] = (r - 0.5) / 0.5;
        }

        let input_tensor = Tensor::from_array((
            [1usize, 3, target_h as usize, target_w as usize],
            chw.into_boxed_slice(),
        ))
        .map_err(|e| format!("PP-OCR rec tensor error: {e}"))?;

        let outputs = self
            .rec_session
            .run(ort::inputs![input_tensor])
            .map_err(|e| format!("PP-OCR rec run error: {e}"))?;

        let (shape, logits) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("PP-OCR rec output error: {e}"))?;
        if shape.len() != 3 {
            return Ok((String::new(), 0.0));
        }
        let seq_len = shape[1] as usize;
        let num_classes = shape[2] as usize;
        Ok(ctc_greedy_decode(logits, seq_len, num_classes, &self.dict))
    }
}

impl OcrBackend for PpOcrOnnxBackend {
    fn engine_name(&self) -> &'static str {
        "ppocr-onnx"
    }

    fn recognize_image(
        &mut self,
        rgb: &[u8],
        width: u32,
        height: u32,
        _lang_hint: &str,
    ) -> Result<Vec<OcrTextBox>, String> {
        let expected = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(3);
        if width == 0 || height == 0 || rgb.len() < expected {
            return Ok(Vec::new());
        }
        let Some(img) = RgbImage::from_raw(width, height, rgb[..expected].to_vec()) else {
            return Ok(Vec::new());
        };

        let boxes = self.detect_boxes(&img)?;
        let mut results = Vec::with_capacity(boxes.len());
        for (x0, y0, x1, y1) in boxes {
            let bw = x1.saturating_sub(x0);
            let bh = y1.saturating_sub(y0);
            if bw < 6 || bh < 6 {
                continue;
            }
            let crop = image::imageops::crop_imm(&img, x0, y0, bw, bh).to_image();
            if let Ok((text, conf)) = self.recognize_crop(&crop) {
                let trimmed = text.trim();
                if !trimmed.is_empty() && conf >= 0.55 {
                    results.push(OcrTextBox {
                        text: trimmed.to_string(),
                        confidence: conf,
                        x: (x0 as f32 / width as f32).clamp(0.0, 1.0),
                        y: (y0 as f32 / height as f32).clamp(0.0, 1.0),
                        width: (bw as f32 / width as f32).clamp(0.0, 1.0),
                        height: (bh as f32 / height as f32).clamp(0.0, 1.0),
                    });
                }
            }
        }
        Ok(results)
    }
}

pub(crate) fn extract_dbnet_boxes(
    prob: &[f32],
    map_w: u32,
    map_h: u32,
    orig_w: u32,
    orig_h: u32,
) -> Vec<(u32, u32, u32, u32)> {
    const BIN_THRESH: f32 = 0.30;
    const BOX_THRESH: f32 = 0.52;
    const UNCLIP_RATIO: f32 = 1.6;

    let w = map_w as usize;
    let h = map_h as usize;
    let mut visited = vec![false; w * h];
    let mut queue = VecDeque::new();
    let mut boxes = Vec::new();

    for sy in 0..h {
        for sx in 0..w {
            let start = sy * w + sx;
            if visited[start] || prob[start] < BIN_THRESH {
                continue;
            }
            visited[start] = true;
            queue.push_back((sx, sy));

            let mut min_x = sx;
            let mut max_x = sx;
            let mut min_y = sy;
            let mut max_y = sy;
            let mut score_sum = 0.0f32;
            let mut count = 0usize;

            while let Some((cx, cy)) = queue.pop_front() {
                let idx = cy * w + cx;
                score_sum += prob[idx];
                count += 1;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);

                let y_lo = cy.saturating_sub(1);
                let y_hi = (cy + 1).min(h - 1);
                let x_lo = cx.saturating_sub(1);
                let x_hi = (cx + 1).min(w - 1);
                for ny in y_lo..=y_hi {
                    for nx in x_lo..=x_hi {
                        let nidx = ny * w + nx;
                        if !visited[nidx] && prob[nidx] >= BIN_THRESH {
                            visited[nidx] = true;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }

            let bw = (max_x - min_x + 1) as f32;
            let bh = (max_y - min_y + 1) as f32;
            if count < 12 || bw < 4.0 || bh < 4.0 {
                continue;
            }
            let mean_score = score_sum / (count as f32);
            if mean_score < BOX_THRESH {
                continue;
            }

            // DBNet rectangular unclip expansion
            let delta = (bw * bh * UNCLIP_RATIO) / (2.0 * (bw + bh));
            let ux0 = ((min_x as f32 - delta * 1.15).max(0.0) / map_w as f32 * orig_w as f32)
                .floor() as u32;
            let uy0 =
                ((min_y as f32 - delta).max(0.0) / map_h as f32 * orig_h as f32).floor() as u32;
            let ux1 = (((max_x + 1) as f32 + delta * 1.15).min(map_w as f32) / map_w as f32
                * orig_w as f32)
                .ceil() as u32;
            let uy1 = (((max_y + 1) as f32 + delta).min(map_h as f32) / map_h as f32
                * orig_h as f32)
                .ceil() as u32;

            boxes.push((
                ux0.min(orig_w),
                uy0.min(orig_h),
                ux1.min(orig_w),
                uy1.min(orig_h),
            ));
        }
    }

    // Sort top-to-bottom, then left-to-right
    boxes.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    boxes
}

pub(crate) fn ctc_greedy_decode(
    logits: &[f32],
    seq_len: usize,
    num_classes: usize,
    dict: &[String],
) -> (String, f32) {
    if seq_len == 0 || num_classes <= 1 || logits.len() < seq_len * num_classes {
        return (String::new(), 0.0);
    }
    let mut out = String::new();
    let mut conf_sum = 0.0f32;
    let mut char_count = 0usize;
    let mut prev_idx = 0usize;

    for t in 0..seq_len {
        let row = &logits[t * num_classes..(t + 1) * num_classes];
        let mut best_idx = 0usize;
        let mut best_val = f32::NEG_INFINITY;
        for (i, &v) in row.iter().enumerate() {
            if v > best_val {
                best_val = v;
                best_idx = i;
            }
        }
        if best_idx != 0 && best_idx != prev_idx {
            let dict_idx = best_idx - 1;
            if let Some(ch) = dict.get(dict_idx) {
                out.push_str(ch);
                conf_sum += best_val.clamp(0.0, 1.0);
                char_count += 1;
            }
        }
        prev_idx = best_idx;
    }

    let avg_conf = if char_count > 0 {
        conf_sum / (char_count as f32)
    } else {
        0.0
    };
    (out, avg_conf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbnet_box_extraction_and_ctc_greedy_decode() {
        // 1. Test DBNet connected-component + unclip box extraction on a 64x32 probability map
        let map_w = 64u32;
        let map_h = 32u32;
        let mut prob = vec![0.02f32; (map_w * map_h) as usize];
        for y in 12..18 {
            for x in 14..50 {
                prob[(y * map_w + x) as usize] = 0.88;
            }
        }
        let boxes = extract_dbnet_boxes(&prob, map_w, map_h, 640, 320);
        assert_eq!(boxes.len(), 1);
        let (x0, y0, x1, y1) = boxes[0];
        assert!(x0 < 140 && x1 > 500 && y0 < 120 && y1 > 180);

        // 2. Test CTC greedy collapse (0 = blank, repeated tokens collapsed)
        let dict = vec!["傅".to_string(), "总".to_string(), " ".to_string()];
        // Sequence: [blank, '傅', '傅', blank, '总'] -> "傅总"
        let logits: Vec<f32> = vec![
            0.95, 0.02, 0.02, 0.01, // blank
            0.03, 0.92, 0.03, 0.02, // 傅
            0.04, 0.90, 0.04, 0.02, // 傅 (collapsed)
            0.94, 0.02, 0.02, 0.02, // blank
            0.02, 0.03, 0.93, 0.02, // 总
        ];
        let (decoded, conf) = ctc_greedy_decode(&logits, 5, 4, &dict);
        assert_eq!(decoded, "傅总");
        assert!(conf > 0.90);
    }
}
