#[cfg(target_os = "macos")]
pub mod macos_vision;
pub mod pipeline;
pub mod ppocr_onnx;
pub mod signs;

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct OcrTextBox {
    pub text: String,
    pub confidence: f32,
    /// Normalized [0.0..1.0] top-left X within the input image
    pub x: f32,
    /// Normalized [0.0..1.0] top-left Y within the input image
    pub y: f32,
    /// Normalized [0.0..1.0] width within the input image
    pub width: f32,
    /// Normalized [0.0..1.0] height within the input image
    pub height: f32,
}

impl OcrTextBox {
    pub fn center_x(&self) -> f32 {
        self.x + self.width * 0.5
    }
    pub fn center_y(&self) -> f32 {
        self.y + self.height * 0.5
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HardsubBox {
    /// Distance from bottom of video frame to bottom of hardsub band (% of video height)
    pub bottom_pct: f32,
    /// Height of hardsub band (% of video height)
    pub height_pct: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct LockedRoiInfo {
    /// Normalized [0..1] top Y in full video frame
    pub y_top: f32,
    /// Normalized [0..1] height in full video frame
    pub height: f32,
    pub hardsub_box: HardsubBox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub original: String,
    pub translated: String,
    pub x_pct: f32,
    pub y_pct: f32,
    pub w_pct: f32,
    pub h_pct: f32,
    pub fg_hex: String,
    pub bg_hex: String,
    pub solid_plate: bool,
    pub font_style: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrStatus {
    pub native_vision: bool,
    pub ppocr_installed: bool,
    pub ready: bool,
    pub engine_name: String,
}

pub trait OcrBackend: Send {
    fn engine_name(&self) -> &'static str;
    fn recognize_image(
        &mut self,
        rgb: &[u8],
        width: u32,
        height: u32,
        lang_hint: &str,
    ) -> Result<Vec<OcrTextBox>, String>;
}

pub fn check_ocr_status(data_dir: &Path) -> OcrStatus {
    let native_vision = cfg!(target_os = "macos");
    let ppocr_installed = ppocr_onnx::is_ppocr_installed(data_dir);
    let ready = native_vision || ppocr_installed;
    let engine_name = if native_vision {
        "apple-vision".to_string()
    } else {
        "ppocr-onnx".to_string()
    };
    OcrStatus {
        native_vision,
        ppocr_installed,
        ready,
        engine_name,
    }
}

pub fn is_ocr_ready(data_dir: &Path) -> bool {
    cfg!(target_os = "macos") || ppocr_onnx::is_ppocr_installed(data_dir)
}

pub fn create_ocr_backend(
    data_dir: &Path,
    min_text_height: f32,
) -> Result<Box<dyn OcrBackend>, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = data_dir;
        return Ok(Box::new(macos_vision::AppleVisionBackend::new(
            min_text_height,
        )));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = min_text_height;
        Ok(Box::new(ppocr_onnx::PpOcrOnnxBackend::load(data_dir)?))
    }
}
