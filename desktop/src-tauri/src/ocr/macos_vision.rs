use crate::ocr::{OcrBackend, OcrTextBox};
use serde::Deserialize;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" {
    fn ytcs_vision_recognize_rgb(
        rgb_ptr: *const u8,
        width: u32,
        height: u32,
        langs_csv: *const c_char,
        min_text_height: f32,
    ) -> *mut c_char;

    fn ytcs_vision_free_str(ptr: *mut c_char);
}

#[derive(Deserialize)]
struct RawVisionItem {
    text: String,
    confidence: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

pub struct AppleVisionBackend {
    min_text_height: f32,
}

impl AppleVisionBackend {
    pub fn new(min_text_height: f32) -> Self {
        Self { min_text_height }
    }

    fn build_langs_csv(lang_hint: &str) -> &'static str {
        match lang_hint.trim().to_lowercase().as_str() {
            "zh" | "zh-cn" | "zh-tw" | "chinese" => "zh-Hans,zh-Hant,en-US",
            "ja" | "japanese" => "ja-JP,en-US,zh-Hans",
            "ko" | "korean" => "ko-KR,en-US,zh-Hans",
            "vi" | "vietnamese" => "vi-VN,en-US",
            "en" | "english" => "en-US,zh-Hans,ja-JP,ko-KR,vi-VN",
            _ => "zh-Hans,zh-Hant,ja-JP,ko-KR,vi-VN,en-US",
        }
    }
}

impl OcrBackend for AppleVisionBackend {
    fn engine_name(&self) -> &'static str {
        "apple-vision"
    }

    fn recognize_image(
        &mut self,
        rgb: &[u8],
        width: u32,
        height: u32,
        lang_hint: &str,
    ) -> Result<Vec<OcrTextBox>, String> {
        let expected = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(3);
        if width == 0 || height == 0 || rgb.len() < expected {
            return Ok(Vec::new());
        }

        let csv = CString::new(Self::build_langs_csv(lang_hint))
            .unwrap_or_else(|_| CString::new("zh-Hans,zh-Hant,en-US").unwrap());

        let raw_json = unsafe {
            let ptr = ytcs_vision_recognize_rgb(
                rgb.as_ptr(),
                width,
                height,
                csv.as_ptr(),
                self.min_text_height,
            );
            if ptr.is_null() {
                return Ok(Vec::new());
            }
            let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            ytcs_vision_free_str(ptr);
            s
        };

        let parsed: Vec<RawVisionItem> = serde_json::from_str(&raw_json).unwrap_or_default();
        Ok(parsed
            .into_iter()
            .filter(|it| !it.text.trim().is_empty())
            .map(|it| OcrTextBox {
                text: it.text.trim().to_string(),
                confidence: it.confidence,
                x: it.x.clamp(0.0, 1.0),
                y: it.y.clamp(0.0, 1.0),
                width: it.width.clamp(0.0, 1.0),
                height: it.height.clamp(0.0, 1.0),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apple_vision_recognizes_synthetic_block_letters() {
        // Render bold white block text "HI 2026" on a dark 320x80 RGB banner
        let w = 320usize;
        let h = 80usize;
        let mut rgb = vec![18u8; w * h * 3];

        // 5x7 bitmap glyphs scaled by 4x (20x28 px)
        let glyphs: [(&str, [u8; 7]); 6] = [
            ("H", [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
            ("I", [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111]),
            ("2", [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
            ("0", [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
            ("2", [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
            ("6", [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        ];

        let scale = 4usize;
        let mut cursor_x = 40usize;
        for (idx, (_, rows)) in glyphs.iter().enumerate() {
            if idx == 2 {
                cursor_x += 16; // space between HI and 2026
            }
            for (gy, &row_bits) in rows.iter().enumerate() {
                for gx in 0..5usize {
                    if (row_bits >> (4 - gx)) & 1 == 1 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cursor_x + gx * scale + sx;
                                let py = 26 + gy * scale + sy;
                                let off = (py * w + px) * 3;
                                rgb[off] = 250;
                                rgb[off + 1] = 250;
                                rgb[off + 2] = 250;
                            }
                        }
                    }
                }
            }
            cursor_x += 5 * scale + 8;
        }

        let mut backend = AppleVisionBackend::new(0.05);
        let boxes = backend
            .recognize_image(&rgb, w as u32, h as u32, "en")
            .expect("Vision OCR FFI call should succeed");
        assert!(!boxes.is_empty(), "Expected Apple Vision to detect synthetic text");
        let combined: String = boxes.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            combined.contains("2026") || combined.to_uppercase().contains("HI"),
            "Unexpected Vision OCR output: {combined}"
        );
    }
}
