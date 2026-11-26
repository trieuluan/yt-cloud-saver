use ort::session::Session;
use ort::value::Tensor;
use std::fs;
use std::path::{Path, PathBuf};

pub const YAMNET_MODEL_URL: &str =
    "https://huggingface.co/andrelgomes/yamnet-onnx/resolve/main/yamnet.onnx";

pub fn yamnet_model_path(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join("yamnet.onnx")
}

pub fn is_yamnet_installed(data_dir: &Path) -> bool {
    yamnet_model_path(data_dir).is_file()
}

pub async fn ensure_yamnet_model(data_dir: &Path) -> Result<PathBuf, String> {
    let dest = yamnet_model_path(data_dir);
    if dest.is_file() {
        return Ok(dest);
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    crate::tools::download_to_file(YAMNET_MODEL_URL, &dest).await?;
    Ok(dest)
}

pub fn remove_yamnet_model(data_dir: &Path) -> Result<(), String> {
    let dest = yamnet_model_path(data_dir);
    if dest.is_file() {
        fs::remove_file(dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// AudioSet YAMNet indices:
// 0 = Speech
// 1 = Male speech, man speaking
// 2 = Female speech, woman speaking
// 27..35 = Singing / Choir / Yodeling / Chant / Mantra / Male/Female singing
// 132 = Music
const SPEECH_IDX: usize = 0;
const MALE_SPEECH_IDX: usize = 1;
const FEMALE_SPEECH_IDX: usize = 2;
const SINGING_INDICES: &[usize] = &[27, 28, 29, 30, 31, 32, 33, 34, 35];
const MUSIC_IDX: usize = 132;

#[derive(Debug, Clone, Copy)]
pub struct YamnetFrameScore {
    pub male: f32,
    pub female: f32,
}

pub fn analyze_pcm_gender_frames(
    data_dir: &Path,
    samples_f32: &[f32],
) -> Result<Vec<YamnetFrameScore>, String> {
    let model_path = yamnet_model_path(data_dir);
    if !model_path.is_file() || samples_f32.len() < 16_000 {
        return Ok(Vec::new());
    }
    let env = ort::init()
        .with_name("ytcs_yamnet")
        .build()
        .map_err(|e| format!("ORT env error: {e}"))?;
    let mut session = Session::builder(&env)
        .map_err(|e| format!("ORT session builder error: {e}"))?
        .commit_from_file(&model_path)
        .map_err(|e| format!("Failed to load yamnet.onnx: {e}"))?;
    let n_samples = samples_f32.len();
    let input_tensor = Tensor::from_array(([n_samples], samples_f32.to_vec().into_boxed_slice()))
        .map_err(|e| format!("Failed to create input tensor: {e}"))?;
    let outputs = session
        .run(ort::inputs![input_tensor])
        .map_err(|e| format!("YAMNet inference failed: {e}"))?;
    let (shape, scores) = outputs[0]
        .try_extract_tensor::<f32>()
        .map_err(|e| format!("Failed to extract YAMNet scores: {e}"))?;
    if shape.len() != 2 || shape[1] < 133 {
        return Ok(Vec::new());
    }
    let num_frames = shape[0] as usize;
    let num_classes = shape[1] as usize;
    let mut out = Vec::with_capacity(num_frames);
    for f in 0..num_frames {
        let row = &scores[f * num_classes..(f + 1) * num_classes];
        out.push(YamnetFrameScore {
            male: row[MALE_SPEECH_IDX],
            female: row[FEMALE_SPEECH_IDX],
        });
    }
    Ok(out)
}

pub fn filter_music_and_singing_wav(
    data_dir: &Path,
    wav_path: &Path,
    sensitivity: &str,
) -> Result<usize, String> {
    let model_path = yamnet_model_path(data_dir);
    if !model_path.is_file() {
        return Err("yamnet.onnx is not installed".to_string());
    }

    let mut reader =
        hound::WavReader::open(wav_path).map_err(|e| format!("Failed to open WAV: {e}"))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != 16_000 {
        return Err(format!(
            "Expected 16kHz mono WAV, got {}Hz {}ch",
            spec.sample_rate, spec.channels
        ));
    }

    let mut pcm_i16: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read WAV samples: {e}"))?;

    if pcm_i16.len() < 16_000 {
        return Ok(0);
    }

    let samples_f32: Vec<f32> = pcm_i16.iter().map(|&s| s as f32 / 32768.0).collect();

    let env = ort::init()
        .with_name("ytcs_yamnet")
        .build()
        .map_err(|e| format!("ORT env error: {e}"))?;

    let mut session = Session::builder(&env)
        .map_err(|e| format!("ORT session builder error: {e}"))?
        .commit_from_file(&model_path)
        .map_err(|e| format!("Failed to load yamnet.onnx: {e}"))?;

    let n_samples = samples_f32.len();
    let input_tensor = Tensor::from_array(([n_samples], samples_f32.into_boxed_slice()))
        .map_err(|e| format!("Failed to create input tensor: {e}"))?;

    let outputs = session
        .run(ort::inputs![input_tensor])
        .map_err(|e| format!("YAMNet inference failed: {e}"))?;

    let (shape, scores) = outputs[0]
        .try_extract_tensor::<f32>()
        .map_err(|e| format!("Failed to extract YAMNet scores: {e}"))?;

    if shape.len() != 2 || shape[1] < 133 {
        return Err(format!("Unexpected YAMNet output shape: {:?}", shape));
    }
    let num_frames = shape[0] as usize;
    let num_classes = shape[1] as usize;
    if num_frames == 0 {
        return Ok(0);
    }

    // Ratio threshold: keep frame if Speech is strong enough relative to Singing/Music
    let ratio = match sensitivity {
        "low" => 1.8_f32,
        "high" => 0.75_f32,
        _ => 1.15_f32,
    };

    let hop_samples: usize = 7680; // 0.48s at 16kHz
    let frame_samples: usize = 15360; // 0.96s at 16kHz
    let mut keep_frame = vec![true; num_frames];

    for f in 0..num_frames {
        let row = &scores[f * num_classes..(f + 1) * num_classes];
        let speech_score = row[SPEECH_IDX];
        let singing_max = SINGING_INDICES
            .iter()
            .map(|&idx| row[idx])
            .fold(0.0_f32, f32::max);
        let music_score = row[MUSIC_IDX];
        let non_speech = singing_max.max(music_score * 0.85);
        if non_speech > 0.25 && non_speech > speech_score * ratio {
            keep_frame[f] = false;
        }
    }

    // Dilate kept speech frames by 1 frame (~0.48s) on each side so word onsets/tails aren't clipped
    let mut keep_dilated = keep_frame.clone();
    for f in 0..num_frames {
        if keep_frame[f] {
            if f > 0 {
                keep_dilated[f - 1] = true;
            }
            if f + 1 < num_frames {
                keep_dilated[f + 1] = true;
            }
        }
    }

    let mut muted_frames = 0usize;
    for (f, &keep) in keep_dilated.iter().enumerate() {
        if !keep {
            muted_frames += 1;
            let start = f * hop_samples;
            let end = (start + frame_samples).min(pcm_i16.len());
            for s in &mut pcm_i16[start..end] {
                *s = 0;
            }
        }
    }

    if muted_frames > 0 {
        let mut writer = hound::WavWriter::create(wav_path, spec)
            .map_err(|e| format!("Failed to write filtered WAV: {e}"))?;
        for s in pcm_i16 {
            writer
                .write_sample(s)
                .map_err(|e| format!("Failed to write WAV sample: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize filtered WAV: {e}"))?;
    }

    Ok(muted_frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yamnet_onnx_shapes() {
        let path = Path::new("/tmp/yamnet_check.onnx");
        if !path.is_file() {
            return;
        }
        let env = ort::init()
            .with_name("ytcs_yamnet_test")
            .build()
            .expect("env");
        let mut session = Session::builder(&env)
            .expect("session builder")
            .commit_from_file(path)
            .expect("load yamnet.onnx");
        let fake_wav = vec![0.0_f32; 32_000];
        let tensor = Tensor::from_array(([32_000usize], fake_wav.into_boxed_slice())).unwrap();
        let outputs = session.run(ort::inputs![tensor]).expect("run yamnet");
        let (shape, _) = outputs[0].try_extract_tensor::<f32>().unwrap();
        println!("YAMNet output shape for 2s audio: {:?}", shape);
        assert_eq!(shape.len(), 2);
        assert_eq!(shape[1], 521);
    }
}
