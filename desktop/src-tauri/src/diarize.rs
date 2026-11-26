use crate::translate::SubtitleCue;
use ort::session::Session;
use ort::value::Tensor;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};

pub const WESPEAKER_MODEL_URL: &str =
    "https://huggingface.co/csukuangfj/speaker-embedding-models/resolve/main/wespeaker_en_voxceleb_resnet34_LM.onnx";

pub fn wespeaker_model_path(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join("wespeaker_voxceleb.onnx")
}

pub fn is_wespeaker_installed(data_dir: &Path) -> bool {
    wespeaker_model_path(data_dir).is_file()
}

pub async fn ensure_wespeaker_model(data_dir: &Path) -> Result<PathBuf, String> {
    let dest = wespeaker_model_path(data_dir);
    if dest.is_file() {
        return Ok(dest);
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    crate::tools::download_to_file(WESPEAKER_MODEL_URL, &dest).await?;
    Ok(dest)
}

pub fn remove_wespeaker_model(data_dir: &Path) -> Result<(), String> {
    let dest = wespeaker_model_path(data_dir);
    if dest.is_file() {
        fs::remove_file(dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// Computes Kaldi-style 80-bin log-Mel filterbank features `[num_frames, 80]` with CMVN.
fn compute_fbank_80(pcm: &[f32]) -> Option<(usize, Vec<f32>)> {
    const SR: f32 = 16_000.0;
    const WIN_LEN: usize = 400; // 25ms
    const HOP_LEN: usize = 160; // 10ms
    const N_FFT: usize = 512;
    const N_MELS: usize = 80;

    if pcm.len() < WIN_LEN + HOP_LEN {
        return None;
    }

    let num_frames = (pcm.len() - WIN_LEN) / HOP_LEN + 1;
    if num_frames < 5 {
        return None;
    }

    // Povey window: hann^0.85
    let window: Vec<f32> = (0..WIN_LEN)
        .map(|n| {
            let hann = 0.5 - 0.5 * (2.0 * PI * n as f32 / (WIN_LEN - 1) as f32).cos();
            hann.powf(0.85)
        })
        .collect();

    // 80 triangular Mel filterbanks across 257 FFT bins (20 Hz .. 7600 Hz)
    let n_bins = N_FFT / 2 + 1;
    let mel_low = hz_to_mel(20.0);
    let mel_high = hz_to_mel(7600.0);
    let mel_points: Vec<f32> = (0..=N_MELS + 1)
        .map(|i| mel_to_hz(mel_low + (mel_high - mel_low) * (i as f32) / (N_MELS + 1) as f32))
        .collect();
    let bin_freqs: Vec<f32> = (0..n_bins)
        .map(|k| (k as f32) * SR / (N_FFT as f32))
        .collect();

    let mut mel_filters = vec![0.0_f32; N_MELS * n_bins];
    for m in 0..N_MELS {
        let f_left = mel_points[m];
        let f_center = mel_points[m + 1];
        let f_right = mel_points[m + 2];
        for (k, &freq) in bin_freqs.iter().enumerate() {
            let w = if freq >= f_left && freq <= f_center && f_center > f_left {
                (freq - f_left) / (f_center - f_left)
            } else if freq > f_center && freq <= f_right && f_right > f_center {
                (f_right - freq) / (f_right - f_center)
            } else {
                0.0
            };
            mel_filters[m * n_bins + k] = w;
        }
    }

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N_FFT);
    let mut buf = vec![Complex::new(0.0_f32, 0.0_f32); N_FFT];
    let mut power = vec![0.0_f32; n_bins];
    let mut feats = vec![0.0_f32; num_frames * N_MELS];

    for f in 0..num_frames {
        let start = f * HOP_LEN;
        let mut prev_s = if start > 0 { pcm[start - 1] } else { pcm[start] };
        for i in 0..WIN_LEN {
            let s = pcm[start + i];
            let pre = s - 0.97 * prev_s;
            prev_s = s;
            buf[i] = Complex::new(pre * window[i], 0.0);
        }
        for b in &mut buf[WIN_LEN..N_FFT] {
            *b = Complex::new(0.0, 0.0);
        }
        fft.process(&mut buf);
        for k in 0..n_bins {
            power[k] = buf[k].norm_sqr();
        }
        for m in 0..N_MELS {
            let row = &mel_filters[m * n_bins..(m + 1) * n_bins];
            let mut energy = 0.0_f32;
            for k in 0..n_bins {
                energy += power[k] * row[k];
            }
            feats[f * N_MELS + m] = energy.max(1e-10).ln();
        }
    }

    // Per-utterance mean normalization (CMVN)
    for m in 0..N_MELS {
        let mut sum = 0.0_f32;
        for f in 0..num_frames {
            sum += feats[f * N_MELS + m];
        }
        let mean = sum / (num_frames as f32);
        for f in 0..num_frames {
            feats[f * N_MELS + m] -= mean;
        }
    }

    Some((num_frames, feats))
}

/// Estimates median fundamental frequency F0 (Hz) for voiced speech segments.
fn estimate_median_f0_hz(pcm: &[f32]) -> Option<f32> {
    const WIN: usize = 400; // 25ms
    const HOP: usize = 160; // 10ms
    const MIN_LAG: usize = 50; // 320 Hz
    const MAX_LAG: usize = 213; // 75 Hz
    if pcm.len() < WIN + HOP {
        return None;
    }
    let mut f0s = Vec::new();
    let mut pos = 0usize;
    while pos + WIN <= pcm.len() {
        let frame = &pcm[pos..pos + WIN];
        let energy: f32 = frame.iter().map(|&x| x * x).sum();
        if energy > 0.015 {
            let mut best_lag = 0usize;
            let mut best_corr = 0.0_f32;
            for lag in MIN_LAG..=MAX_LAG.min(WIN / 2) {
                let mut num = 0.0_f32;
                let mut den_a = 0.0_f32;
                let mut den_b = 0.0_f32;
                for i in 0..(WIN - lag) {
                    let a = frame[i];
                    let b = frame[i + lag];
                    num += a * b;
                    den_a += a * a;
                    den_b += b * b;
                }
                let denom = (den_a * den_b).sqrt();
                if denom > 1e-6 {
                    let corr = num / denom;
                    if corr > best_corr {
                        best_corr = corr;
                        best_lag = lag;
                    }
                }
            }
            if best_corr > 0.58 && best_lag > 0 {
                f0s.push(16_000.0 / best_lag as f32);
            }
        }
        pos += HOP;
    }
    if f0s.len() < 3 {
        return None;
    }
    f0s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(f0s[f0s.len() / 2])
}

fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

struct SpeakerCluster {
    label: String,
    centroid: Vec<f32>,
    embed_count: usize,
    f0_samples: Vec<f32>,
    male_weight: f32,
    female_weight: f32,
}

impl SpeakerCluster {
    fn gender_tag(&self) -> &'static str {
        let mut median_f0 = None;
        if !self.f0_samples.is_empty() {
            let mut sorted = self.f0_samples.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            median_f0 = Some(sorted[sorted.len() / 2]);
        }

        let ym_male = self.male_weight;
        let ym_female = self.female_weight;
        match median_f0 {
            Some(f0) if f0 < 162.0 && ym_male >= ym_female * 0.75 => "Nam",
            Some(f0) if f0 > 176.0 && ym_female >= ym_male * 0.75 => "Nữ",
            Some(f0) if f0 < 145.0 => "Nam",
            Some(f0) if f0 > 195.0 => "Nữ",
            _ => {
                if ym_male > ym_female * 1.4 && ym_male > 0.3 {
                    "Nam"
                } else if ym_female > ym_male * 1.4 && ym_female > 0.3 {
                    "Nữ"
                } else {
                    ""
                }
            }
        }
    }

    fn format_hint(&self) -> String {
        let g = self.gender_tag();
        if g.is_empty() {
            format!("[Speaker {}]", self.label)
        } else {
            format!("[{g} · {}]", self.label)
        }
    }

    fn format_code(&self) -> String {
        let prefix = match self.gender_tag() {
            "Nữ" => "F",
            "Nam" => "M",
            _ => "U",
        };
        format!("{prefix}:{}", self.label)
    }
}

pub struct SpeakerTracker {
    clusters: Vec<SpeakerCluster>,
}

impl SpeakerTracker {
    pub fn new() -> Self {
        Self {
            clusters: Vec::new(),
        }
    }

    fn next_label(idx: usize) -> String {
        if idx < 26 {
            ((b'A' + idx as u8) as char).to_string()
        } else {
            format!("S{}", idx + 1)
        }
    }

    /// Assigns `(prompt_hint, speaker_code)` for each cue in a chunk WAV.
    pub fn diarize_chunk(
        &mut self,
        data_dir: &Path,
        wav_path: &Path,
        offset_ms: u64,
        cues: &[SubtitleCue],
    ) -> Result<Vec<(String, Option<String>)>, String> {
        let model_path = wespeaker_model_path(data_dir);
        if !model_path.is_file() || cues.is_empty() {
            return Ok(vec![(String::new(), None); cues.len()]);
        }

        let mut reader =
            hound::WavReader::open(wav_path).map_err(|e| format!("Failed to open WAV: {e}"))?;
        let spec = reader.spec();
        if spec.channels != 1 || spec.sample_rate != 16_000 {
            return Ok(vec![(String::new(), None); cues.len()]);
        }
        let pcm_f32: Vec<f32> = reader
            .samples::<i16>()
            .filter_map(Result::ok)
            .map(|s| s as f32 / 32768.0)
            .collect();
        if pcm_f32.len() < 8_000 {
            return Ok(vec![(String::new(), None); cues.len()]);
        }

        let yamnet_frames = crate::yamnet::analyze_pcm_gender_frames(data_dir, &pcm_f32)
            .unwrap_or_default();

        let env = ort::init()
            .with_name("ytcs_wespeaker")
            .build()
            .map_err(|e| format!("ORT env error: {e}"))?;
        let mut session = Session::builder(&env)
            .map_err(|e| format!("ORT session builder error: {e}"))?
            .commit_from_file(&model_path)
            .map_err(|e| format!("Failed to load wespeaker_voxceleb.onnx: {e}"))?;

        let mut assigned_cluster: Vec<Option<usize>> = vec![None; cues.len()];

        for (i, cue) in cues.iter().enumerate() {
            let rel_start_ms = cue.start_ms.saturating_sub(offset_ms);
            let rel_end_ms = cue.end_ms.saturating_sub(offset_ms);
            let s_idx = ((rel_start_ms as usize) * 16).min(pcm_f32.len());
            let e_idx = ((rel_end_ms as usize) * 16).min(pcm_f32.len());
            if e_idx <= s_idx {
                continue;
            }
            let slice = &pcm_f32[s_idx..e_idx];
            let dur_ms = rel_end_ms.saturating_sub(rel_start_ms);
            if dur_ms < 350 {
                continue;
            }
            let rms = (slice.iter().map(|&x| x * x).sum::<f32>() / slice.len() as f32).sqrt();
            if rms < 0.004 {
                continue;
            }

            let Some((n_frames, fbank)) = compute_fbank_80(slice) else {
                continue;
            };
            let tensor = Tensor::from_array(([1usize, n_frames, 80usize], fbank.into_boxed_slice()))
                .map_err(|e| format!("Fbank tensor error: {e}"))?;
            let outputs = session
                .run(ort::inputs![tensor])
                .map_err(|e| format!("WeSpeaker inference failed: {e}"))?;
            let (_, emb_slice) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("Failed to extract embedding: {e}"))?;
            let mut emb = emb_slice.to_vec();
            if emb.is_empty() {
                continue;
            }
            l2_normalize(&mut emb);

            let threshold = if dur_ms >= 800 { 0.60 } else { 0.52 };
            let mut best_idx = None;
            let mut best_sim = -1.0_f32;
            for (c_idx, cl) in self.clusters.iter().enumerate() {
                if cl.centroid.len() == emb.len() {
                    let sim = cosine_sim(&emb, &cl.centroid);
                    if sim > best_sim {
                        best_sim = sim;
                        best_idx = Some(c_idx);
                    }
                }
            }

            let target_idx = if best_sim >= threshold || self.clusters.len() >= 12 {
                let idx = best_idx.unwrap_or(0);
                // Only update centroid with longer, cleaner utterances
                if dur_ms >= 800 {
                    let cl = &mut self.clusters[idx];
                    let n = cl.embed_count.min(40) as f32;
                    for (c_val, &e_val) in cl.centroid.iter_mut().zip(emb.iter()) {
                        *c_val = (*c_val * n + e_val) / (n + 1.0);
                    }
                    l2_normalize(&mut cl.centroid);
                    cl.embed_count += 1;
                }
                idx
            } else {
                let label = Self::next_label(self.clusters.len());
                self.clusters.push(SpeakerCluster {
                    label,
                    centroid: emb,
                    embed_count: 1,
                    f0_samples: Vec::new(),
                    male_weight: 0.0,
                    female_weight: 0.0,
                });
                self.clusters.len() - 1
            };

            assigned_cluster[i] = Some(target_idx);

            if let Some(f0) = estimate_median_f0_hz(slice) {
                self.clusters[target_idx].f0_samples.push(f0);
            }
            if !yamnet_frames.is_empty() {
                let f_start = (rel_start_ms as usize) / 480;
                let f_end = ((rel_end_ms as usize) / 480 + 1).min(yamnet_frames.len());
                for f in f_start..f_end {
                    self.clusters[target_idx].male_weight += yamnet_frames[f].male;
                    self.clusters[target_idx].female_weight += yamnet_frames[f].female;
                }
            }
        }

        let hints = assigned_cluster
            .into_iter()
            .map(|opt| match opt {
                Some(idx) => (
                    self.clusters[idx].format_hint(),
                    Some(self.clusters[idx].format_code()),
                ),
                None => (String::new(), None),
            })
            .collect();
        Ok(hints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fbank_and_f0_synthetic() {
        // 1 second of 220 Hz sine wave (female range) at 16kHz
        let pcm: Vec<f32> = (0..16_000)
            .map(|n| 0.3 * (2.0 * PI * 220.0 * (n as f32) / 16_000.0).sin())
            .collect();
        let (frames, fbank) = compute_fbank_80(&pcm).expect("fbank");
        assert!(frames > 90);
        assert_eq!(fbank.len(), frames * 80);

        let f0 = estimate_median_f0_hz(&pcm).expect("f0");
        assert!((f0 - 220.0).abs() < 15.0, "expected ~220Hz, got {f0}");

        let model_path = Path::new("/Users/louis/Library/Application Support/com.louis.yt-cloud-saver/models/wespeaker_voxceleb.onnx");
        if model_path.is_file() {
            let env = ort::init().with_name("ytcs_ws_test").build().expect("env");
            let mut session = Session::builder(&env)
                .expect("builder")
                .commit_from_file(model_path)
                .expect("load wespeaker");
            let tensor = Tensor::from_array(([1usize, frames, 80usize], fbank.into_boxed_slice())).expect("tensor");
            let outputs = session.run(ort::inputs![tensor]).expect("run wespeaker");
            let (shape, _) = outputs[0].try_extract_tensor::<f32>().expect("extract");
            println!("WeSpeaker output shape: {:?}", shape);
        }
    }
}
