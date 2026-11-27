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
    locked_gender: Option<&'static str>,
}

impl SpeakerCluster {
    fn gender_tag(&self) -> &'static str {
        if let Some(g) = self.locked_gender {
            return g;
        }
        let mut median_f0 = None;
        if !self.f0_samples.is_empty() {
            let mut sorted = self.f0_samples.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            median_f0 = Some(sorted[sorted.len() / 2]);
        }

        let ym_male = self.male_weight;
        let ym_female = self.female_weight;
        match median_f0 {
            Some(f0) if f0 < 158.0 => "Nam",
            Some(f0) if f0 > 176.0 => "Nữ",
            _ => {
                if ym_male > ym_female * 1.4 && ym_male > 0.3 {
                    "Nam"
                } else if ym_female > ym_male * 1.4 && ym_female > 0.3 {
                    "Nữ"
                } else if let Some(f0) = median_f0 {
                    if f0 < 167.0 {
                        "Nam"
                    } else {
                        "Nữ"
                    }
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
    wespeaker_session: Option<Session>,
    yamnet_session: Option<Session>,
}

impl SpeakerTracker {
    pub fn new() -> Self {
        Self {
            clusters: Vec::new(),
            wespeaker_session: None,
            yamnet_session: None,
        }
    }

    fn next_label(idx: usize) -> String {
        if idx < 26 {
            ((b'A' + idx as u8) as char).to_string()
        } else {
            format!("S{}", idx + 1)
        }
    }

    /// Assigns `(prompt_hint, speaker_code)` using precomputed YAMNet scores when available.
    pub fn diarize_chunk_with_yamnet(
        &mut self,
        data_dir: &Path,
        wav_path: &Path,
        offset_ms: u64,
        cues: &[SubtitleCue],
        precomputed_yamnet: Option<&[crate::yamnet::YamnetFrameScore]>,
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

        let owned_yamnet;
        let yamnet_frames: &[crate::yamnet::YamnetFrameScore] = if let Some(frames) = precomputed_yamnet {
            frames
        } else {
            if self.yamnet_session.is_none() && crate::yamnet::yamnet_model_path(data_dir).is_file() {
                self.yamnet_session = crate::yamnet::load_yamnet_session(data_dir).ok();
            }
            owned_yamnet = match self.yamnet_session.as_mut() {
                Some(ys) => crate::yamnet::analyze_pcm_gender_frames_with_session(ys, &pcm_f32)
                    .unwrap_or_default(),
                None => Vec::new(),
            };
            &owned_yamnet
        };

        if self.wespeaker_session.is_none() {
            let env = crate::yamnet::shared_ort_env()?;
            let s = Session::builder(env)
                .map_err(|e| format!("ORT session builder error: {e}"))?
                .commit_from_file(&model_path)
                .map_err(|e| format!("Failed to load wespeaker_voxceleb.onnx: {e}"))?;
            self.wespeaker_session = Some(s);
        }
        let session = self
            .wespeaker_session
            .as_mut()
            .ok_or_else(|| "WeSpeaker session unavailable".to_string())?;

        struct CueSample {
            idx: usize,
            dur_ms: u64,
            rms: f32,
            emb: Vec<f32>,
            f0: Option<f32>,
            ym_male: f32,
            ym_female: f32,
            speech_strength: f32,
        }

        let mut samples: Vec<CueSample> = Vec::with_capacity(cues.len());
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
            if n_frames < 20 {
                continue;
            }
            let Ok(tensor) = Tensor::from_array(([1usize, n_frames, 80usize], fbank.into_boxed_slice())) else {
                continue;
            };
            let Ok(outputs) = session.run(ort::inputs![tensor]) else {
                continue;
            };
            let Ok((_, emb_slice)) = outputs[0].try_extract_tensor::<f32>() else {
                continue;
            };
            let mut emb = emb_slice.to_vec();
            if emb.is_empty() {
                continue;
            }
            l2_normalize(&mut emb);

            let f0 = estimate_median_f0_hz(slice);
            let mut ym_male = 0.0_f32;
            let mut ym_female = 0.0_f32;
            let mut frame_cnt = 0usize;
            if !yamnet_frames.is_empty() {
                let f_start = (rel_start_ms as usize) / 480;
                let f_end = ((rel_end_ms as usize) / 480 + 1).min(yamnet_frames.len());
                frame_cnt = f_end.saturating_sub(f_start);
                for f in f_start..f_end {
                    ym_male += yamnet_frames[f].male;
                    ym_female += yamnet_frames[f].female;
                }
            }
            let speech_strength = if frame_cnt > 0 {
                (ym_male + ym_female) / frame_cnt as f32
            } else {
                0.0
            };

            samples.push(CueSample {
                idx: i,
                dur_ms,
                rms,
                emb,
                f0,
                ym_male,
                ym_female,
                speech_strength,
            });
        }

        if samples.is_empty() {
            return Ok(vec![(String::new(), None); cues.len()]);
        }

        // Pass 1: Build clean Anchor Clusters via Agglomerative Clustering on long, clean utterances
        let mut anchor_indices: Vec<usize> = samples
            .iter()
            .enumerate()
            .filter(|(_, s)| s.dur_ms >= 1400 && s.rms >= 0.006)
            .map(|(k, _)| k)
            .collect();
        if anchor_indices.is_empty() {
            anchor_indices = (0..samples.len()).collect();
        }

        struct MiniCluster {
            first_sample_idx: usize,
            centroid: Vec<f32>,
            members: Vec<usize>,
        }

        let mut mini_clusters: Vec<MiniCluster> = Vec::new();
        for &s_k in &anchor_indices {
            let s = &samples[s_k];
            // If existing clusters from prior chunks already match strongly, update them directly
            let mut best_existing = None;
            let mut best_sim = -1.0_f32;
            for (c_idx, cl) in self.clusters.iter().enumerate() {
                if cl.centroid.len() == s.emb.len() {
                    let sim = cosine_sim(&s.emb, &cl.centroid);
                    if sim > best_sim {
                        best_sim = sim;
                        best_existing = Some(c_idx);
                    }
                }
            }
            if best_sim >= 0.50 {
                if let Some(c_idx) = best_existing {
                    let cl = &mut self.clusters[c_idx];
                    let n = cl.embed_count.min(40) as f32;
                    for (c_val, &e_val) in cl.centroid.iter_mut().zip(s.emb.iter()) {
                        *c_val = (*c_val * n + e_val) / (n + 1.0);
                    }
                    l2_normalize(&mut cl.centroid);
                    cl.embed_count += 1;
                    if let Some(f0) = s.f0 {
                        cl.f0_samples.push(f0);
                    }
                    cl.male_weight += s.ym_male;
                    cl.female_weight += s.ym_female;
                    continue;
                }
            }
            mini_clusters.push(MiniCluster {
                first_sample_idx: s.idx,
                centroid: s.emb.clone(),
                members: vec![s_k],
            });
        }

        // Agglomerative Hierarchical Clustering (merge closest pair while sim >= 0.43)
        while mini_clusters.len() > 1 {
            let mut best_pair = None;
            let mut best_sim = -1.0_f32;
            for a in 0..mini_clusters.len() {
                for b in (a + 1)..mini_clusters.len() {
                    let sim = cosine_sim(&mini_clusters[a].centroid, &mini_clusters[b].centroid);
                    if sim > best_sim {
                        best_sim = sim;
                        best_pair = Some((a, b));
                    }
                }
            }
            if best_sim < 0.43 {
                break;
            }
            if let Some((a, b)) = best_pair {
                let b_cl = mini_clusters.remove(b);
                let a_cl = &mut mini_clusters[a];
                let wa = a_cl.members.len() as f32;
                let wb = b_cl.members.len() as f32;
                for (ca, &cb) in a_cl.centroid.iter_mut().zip(b_cl.centroid.iter()) {
                    *ca = (*ca * wa + cb * wb) / (wa + wb);
                }
                l2_normalize(&mut a_cl.centroid);
                a_cl.first_sample_idx = a_cl.first_sample_idx.min(b_cl.first_sample_idx);
                a_cl.members.extend(b_cl.members);
            }
        }

        // Merge singleton noise mini-clusters into nearest multi-member cluster if reasonably close
        if mini_clusters.iter().filter(|m| m.members.len() >= 2).count() >= 1 {
            let mut i = 0;
            while i < mini_clusters.len() {
                if mini_clusters[i].members.len() == 1 && mini_clusters.len() > 1 {
                    let mut best_target = None;
                    let mut best_sim = -1.0_f32;
                    for j in 0..mini_clusters.len() {
                        if i != j && mini_clusters[j].members.len() >= 2 {
                            let sim = cosine_sim(&mini_clusters[i].centroid, &mini_clusters[j].centroid);
                            if sim > best_sim {
                                best_sim = sim;
                                best_target = Some(j);
                            }
                        }
                    }
                    if best_sim >= 0.34 {
                        if let Some(j) = best_target {
                            let removed = mini_clusters.remove(i);
                            let target_idx = if j > i { j - 1 } else { j };
                            mini_clusters[target_idx].members.extend(removed.members);
                            continue;
                        }
                    }
                }
                i += 1;
            }
        }

        // Order new anchor clusters chronologically by earliest cue, and register up to max 8 total
        mini_clusters.sort_by_key(|m| m.first_sample_idx);
        for mc in mini_clusters {
            if self.clusters.len() >= 8 {
                break;
            }
            let label = Self::next_label(self.clusters.len());
            let mut f0_samples = Vec::new();
            let mut male_weight = 0.0_f32;
            let mut female_weight = 0.0_f32;
            // Weight cleaner speech higher when computing anchor centroid
            let mut centroid = vec![0.0_f32; mc.centroid.len()];
            for &s_k in &mc.members {
                let s = &samples[s_k];
                let w = 1.0 + s.speech_strength.min(1.0);
                for (c_val, &e_val) in centroid.iter_mut().zip(s.emb.iter()) {
                    *c_val += e_val * w;
                }
                if let Some(f0) = s.f0 {
                    f0_samples.push(f0);
                }
                male_weight += s.ym_male;
                female_weight += s.ym_female;
            }
            l2_normalize(&mut centroid);
            let mut cl = SpeakerCluster {
                label,
                centroid,
                embed_count: mc.members.len(),
                f0_samples,
                male_weight,
                female_weight,
                locked_gender: None,
            };
            if cl.f0_samples.len() >= 5 {
                let g = cl.gender_tag();
                if !g.is_empty() {
                    cl.locked_gender = Some(g);
                }
            }
            self.clusters.push(cl);
        }

        // Pass 2: Assign ALL cues (including short/noisy ones) to the anchored clusters
        let mut assigned_cluster: Vec<Option<usize>> = vec![None; cues.len()];
        for s in &samples {
            let cue_clear_gender = match s.f0 {
                Some(f0) if f0 < 145.0 => Some("Nam"),
                Some(f0) if f0 > 195.0 => Some("Nữ"),
                _ => None,
            };

            let mut best_idx = 0usize;
            let mut best_sim = -1.0_f32;
            let mut best_gender_idx = None;
            let mut best_gender_sim = -1.0_f32;

            for (c_idx, cl) in self.clusters.iter().enumerate() {
                if cl.centroid.len() == s.emb.len() {
                    let sim = cosine_sim(&s.emb, &cl.centroid);
                    if sim > best_sim {
                        best_sim = sim;
                        best_idx = c_idx;
                    }
                    if let Some(cg) = cue_clear_gender {
                        if cl.gender_tag() == cg && sim > best_gender_sim {
                            best_gender_sim = sim;
                            best_gender_idx = Some(c_idx);
                        }
                    }
                }
            }

            let target_idx = if let (Some(cg), Some(g_idx)) = (cue_clear_gender, best_gender_idx) {
                let best_g = self.clusters[best_idx].gender_tag();
                if !best_g.is_empty() && best_g != cg && (best_sim - best_gender_sim <= 0.08 || best_sim < 0.46) {
                    g_idx
                } else {
                    best_idx
                }
            } else {
                best_idx
            };

            assigned_cluster[s.idx] = Some(target_idx);
            if s.dur_ms >= 1000 && best_sim >= 0.48 {
                if let Some(f0) = s.f0 {
                    self.clusters[target_idx].f0_samples.push(f0);
                }
                self.clusters[target_idx].male_weight += s.ym_male;
                self.clusters[target_idx].female_weight += s.ym_female;
                if self.clusters[target_idx].locked_gender.is_none()
                    && self.clusters[target_idx].f0_samples.len() >= 5
                {
                    let g = self.clusters[target_idx].gender_tag();
                    if !g.is_empty() {
                        self.clusters[target_idx].locked_gender = Some(g);
                    }
                }
            }
        }

        // Pass 3: Smooth isolated short (< 1s) A -> X -> A flips when tightly sandwiched
        if assigned_cluster.len() >= 3 {
            let snapshot = assigned_cluster.clone();
            for i in 1..(assigned_cluster.len() - 1) {
                if let (Some(c_prev), Some(_), Some(c_next)) =
                    (snapshot[i - 1], snapshot[i], snapshot[i + 1])
                {
                    if c_prev == c_next && snapshot[i] != Some(c_prev) {
                        let dur_ms = cues[i].end_ms.saturating_sub(cues[i].start_ms);
                        let gap_prev = cues[i].start_ms.saturating_sub(cues[i - 1].end_ms);
                        let gap_next = cues[i + 1].start_ms.saturating_sub(cues[i].end_ms);
                        if dur_ms < 1000 && gap_prev <= 600 && gap_next <= 600 {
                            assigned_cluster[i] = Some(c_prev);
                        }
                    }
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

        let candidate_path = std::env::var_os("YTCS_WESPEAKER_MODEL")
            .map(PathBuf::from)
            .or_else(|| {
                #[cfg(target_os = "macos")]
                {
                    std::env::var_os("HOME").map(|h| {
                        PathBuf::from(h)
                            .join("Library/Application Support/com.louis.yt-cloud-saver/models/wespeaker_voxceleb.onnx")
                    })
                }
                #[cfg(target_os = "windows")]
                {
                    std::env::var_os("APPDATA").map(|a| {
                        PathBuf::from(a)
                            .join("com.louis.yt-cloud-saver/models/wespeaker_voxceleb.onnx")
                    })
                }
                #[cfg(all(unix, not(target_os = "macos")))]
                {
                    std::env::var_os("XDG_DATA_HOME")
                        .map(PathBuf::from)
                        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
                        .map(|d| d.join("com.louis.yt-cloud-saver/models/wespeaker_voxceleb.onnx"))
                }
            });
        if let Some(model_path) = candidate_path.filter(|p| p.is_file()) {
            let env = crate::yamnet::shared_ort_env().expect("env");
            let mut session = Session::builder(env)
                .expect("builder")
                .commit_from_file(&model_path)
                .expect("load wespeaker");
            let tensor = Tensor::from_array(([1usize, frames, 80usize], fbank.into_boxed_slice())).expect("tensor");
            let outputs = session.run(ort::inputs![tensor]).expect("run wespeaker");
            let (shape, _) = outputs[0].try_extract_tensor::<f32>().expect("extract");
            println!("WeSpeaker output shape: {:?}", shape);
        }
    }

    #[test]
    fn test_diarize_chunk_with_yamnet_and_wespeaker() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let data_dir = PathBuf::from(home)
            .join("Library/Application Support/com.louis.yt-cloud-saver");
        if !wespeaker_model_path(&data_dir).is_file() {
            return;
        }
        let wav_path = std::env::temp_dir().join("ytcs_diarize_e2e_test.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        {
            let mut w = hound::WavWriter::create(&wav_path, spec).expect("wav");
            for i in 0..80_000 {
                let t = i as f32 / 16_000.0;
                let s = ((2.0 * PI * 180.0 * t).sin() * 8000.0) as i16;
                w.write_sample(s).unwrap();
            }
            w.finalize().unwrap();
        }
        let mut yamnet_cached = None;
        let precomputed_yamnet = if crate::yamnet::yamnet_model_path(&data_dir).is_file() {
            let (muted, scores) = crate::yamnet::filter_music_and_singing_wav_with_scores(
                &data_dir,
                &wav_path,
                "balanced",
                &mut yamnet_cached,
            )
            .expect("yamnet filter");
            println!("YAMNet muted frames: {muted}, gender frames: {}", scores.len());
            assert!(!scores.is_empty());
            Some(scores)
        } else {
            None
        };
        let cues = vec![
            SubtitleCue {
                start_ms: 0,
                end_ms: 360,
                original: "short".into(),
                translated: String::new(),
                speaker: None,
            },
            SubtitleCue {
                start_ms: 1600,
                end_ms: 4500,
                original: "world".into(),
                translated: String::new(),
                speaker: None,
            },
        ];
        let cues_2 = vec![
            SubtitleCue {
                start_ms: 5000,
                end_ms: 5360,
                original: "short".into(),
                translated: String::new(),
                speaker: None,
            },
            SubtitleCue {
                start_ms: 6600,
                end_ms: 9500,
                original: "world".into(),
                translated: String::new(),
                speaker: None,
            },
        ];
        let mut tracker = SpeakerTracker::new();
        let pairs_1 = tracker
            .diarize_chunk_with_yamnet(&data_dir, &wav_path, 0, &cues, precomputed_yamnet.as_deref())
            .expect("diarize_chunk 1");
        let pairs_2 = tracker
            .diarize_chunk_with_yamnet(&data_dir, &wav_path, 5000, &cues_2, precomputed_yamnet.as_deref())
            .expect("diarize_chunk 2 (cached session)");
        let _ = fs::remove_file(&wav_path);
        println!("diarize_chunk pairs_1: {:?}, pairs_2: {:?}", pairs_1, pairs_2);
        assert!(pairs_1.iter().any(|(_, code)| code.is_some()));
        assert!(pairs_2.iter().any(|(_, code)| code.is_some()));
    }
}
