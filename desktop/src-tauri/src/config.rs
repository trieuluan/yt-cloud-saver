use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiProfile {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomVoice {
    pub id: String,
    pub name: String,
    pub ref_audio: String,
}

fn default_ai_profiles() -> HashMap<String, AiProfile> {
    let mut map = HashMap::new();
    map.insert(
        "gemini".to_string(),
        AiProfile {
            base_url: String::new(),
            api_key: String::new(),
            model: "gemini-3.8-flash".to_string(),
        },
    );
    map.insert(
        "openai".to_string(),
        AiProfile {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
        },
    );
    map.insert(
        "ollama".to_string(),
        AiProfile {
            base_url: "http://localhost:11434/v1".to_string(),
            api_key: String::new(),
            model: "qwen2.5:14b".to_string(),
        },
    );
    map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub port: u16,
    pub host: String,
    pub rclone_remote: String,
    pub group_by_channel: bool,
    pub quality: String,
    #[serde(default)]
    pub cookies_browser: String,
    #[serde(default = "default_ai_provider")]
    pub ai_provider: String,
    #[serde(default)]
    pub ai_base_url: String,
    #[serde(default)]
    pub ai_api_key: String,
    #[serde(default = "default_ai_model")]
    pub ai_model: String,
    #[serde(default = "default_ai_profiles")]
    pub ai_profiles: HashMap<String, AiProfile>,
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
    #[serde(default = "default_true")]
    pub use_whisper_fallback: bool,
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
    #[serde(default = "default_whisper_language")]
    pub whisper_language: String,
    #[serde(default = "default_whisper_mode")]
    pub whisper_mode: String,
    #[serde(default = "default_whisper_cloud_provider")]
    pub whisper_cloud_provider: String,
    #[serde(default = "default_whisper_cloud_base_url")]
    pub whisper_cloud_base_url: String,
    #[serde(default)]
    pub whisper_cloud_api_key: String,
    #[serde(default = "default_whisper_cloud_model")]
    pub whisper_cloud_model: String,
    #[serde(default)]
    pub prefer_whisper_over_auto_cc: bool,
    #[serde(default = "default_true")]
    pub skip_music_whisper: bool,
    #[serde(default = "default_yamnet_sensitivity")]
    pub yamnet_sensitivity: String,
    #[serde(default)]
    pub use_speaker_diarization: bool,
    #[serde(default)]
    pub embed_subtitles: bool,
    #[serde(default = "default_tts_mode")]
    pub tts_mode: String,
    #[serde(default = "default_vieneu_model")]
    pub vieneu_model: String,
    #[serde(default = "default_vieneu_voice")]
    pub vieneu_voice: String,
    #[serde(default)]
    pub vieneu_ref_audio: String,
    #[serde(default)]
    pub custom_voices: Vec<CustomVoice>,
    pub max_concurrent: usize,
    pub keep_local: bool,
    pub temp_dir: String,
}

fn default_ai_provider() -> String {
    "gemini".to_string()
}
fn default_ai_model() -> String {
    "gemini-3.8-flash".to_string()
}
fn default_target_lang() -> String {
    "Vietnamese".to_string()
}
fn default_true() -> bool {
    true
}
fn default_whisper_model() -> String {
    "small".to_string()
}
fn default_whisper_language() -> String {
    "auto".to_string()
}
fn default_whisper_mode() -> String {
    "local".to_string()
}
fn default_whisper_cloud_provider() -> String {
    "groq".to_string()
}
fn default_whisper_cloud_base_url() -> String {
    "https://api.groq.com/openai/v1".to_string()
}
fn default_whisper_cloud_model() -> String {
    "whisper-large-v3-turbo".to_string()
}
fn default_tts_mode() -> String {
    "edge".to_string()
}
fn default_yamnet_sensitivity() -> String {
    "med".to_string()
}
fn default_vieneu_model() -> String {
    "v3turbo".to_string()
}
fn default_vieneu_voice() -> String {
    "Ngọc Huyền".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        let temp_dir = std::env::temp_dir()
            .join("yt-cloud-saver")
            .to_string_lossy()
            .to_string();
        Self {
            port: 4873,
            host: "127.0.0.1".to_string(),
            rclone_remote: "gdrive:YouTube Saved".to_string(),
            group_by_channel: true,
            quality: "best".to_string(),
            cookies_browser: String::new(),
            ai_provider: default_ai_provider(),
            ai_base_url: String::new(),
            ai_api_key: String::new(),
            ai_model: default_ai_model(),
            ai_profiles: default_ai_profiles(),
            target_lang: default_target_lang(),
            use_whisper_fallback: true,
            whisper_model: default_whisper_model(),
            whisper_language: default_whisper_language(),
            whisper_mode: default_whisper_mode(),
            whisper_cloud_provider: default_whisper_cloud_provider(),
            whisper_cloud_base_url: default_whisper_cloud_base_url(),
            whisper_cloud_api_key: String::new(),
            whisper_cloud_model: default_whisper_cloud_model(),
            prefer_whisper_over_auto_cc: false,
            skip_music_whisper: true,
            yamnet_sensitivity: default_yamnet_sensitivity(),
            use_speaker_diarization: false,
            embed_subtitles: false,
            tts_mode: default_tts_mode(),
            vieneu_model: default_vieneu_model(),
            vieneu_voice: default_vieneu_voice(),
            vieneu_ref_audio: String::new(),
            custom_voices: Vec::new(),
            max_concurrent: 2,
            keep_local: false,
            temp_dir,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPatch {
    pub rclone_remote: Option<String>,
    pub group_by_channel: Option<bool>,
    pub quality: Option<String>,
    pub cookies_browser: Option<String>,
    pub ai_provider: Option<String>,
    pub ai_base_url: Option<String>,
    pub ai_api_key: Option<String>,
    pub ai_model: Option<String>,
    pub ai_profiles: Option<HashMap<String, AiProfile>>,
    pub target_lang: Option<String>,
    pub use_whisper_fallback: Option<bool>,
    pub whisper_model: Option<String>,
    pub whisper_language: Option<String>,
    pub whisper_mode: Option<String>,
    pub whisper_cloud_provider: Option<String>,
    pub whisper_cloud_base_url: Option<String>,
    pub whisper_cloud_api_key: Option<String>,
    pub whisper_cloud_model: Option<String>,
    pub prefer_whisper_over_auto_cc: Option<bool>,
    pub skip_music_whisper: Option<bool>,
    pub yamnet_sensitivity: Option<String>,
    pub use_speaker_diarization: Option<bool>,
    pub embed_subtitles: Option<bool>,
    pub tts_mode: Option<String>,
    pub vieneu_model: Option<String>,
    pub vieneu_voice: Option<String>,
    pub vieneu_ref_audio: Option<String>,
    pub custom_voices: Option<Vec<CustomVoice>>,
    pub max_concurrent: Option<usize>,
    pub keep_local: Option<bool>,
}

fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("config.json")
}

pub fn load_config(data_dir: &Path) -> AppConfig {
    let path = config_path(data_dir);
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&raw) {
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn save_config(data_dir: &Path, patch: ConfigPatch) -> Result<AppConfig, String> {
    fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let mut cfg = load_config(data_dir);
    if let Some(remote) = patch.rclone_remote {
        cfg.rclone_remote = remote.trim().to_string();
    }
    if let Some(group) = patch.group_by_channel {
        cfg.group_by_channel = group;
    }
    if let Some(quality) = patch.quality {
        cfg.quality = quality;
    }
    if let Some(cookies) = patch.cookies_browser {
        cfg.cookies_browser = cookies.trim().to_string();
    }
    if let Some(profiles) = patch.ai_profiles {
        for (k, v) in profiles {
            cfg.ai_profiles.insert(
                k,
                AiProfile {
                    base_url: v.base_url.trim().trim_end_matches('/').to_string(),
                    api_key: v.api_key.trim().to_string(),
                    model: v.model.trim().to_string(),
                },
            );
        }
    }
    if let Some(prov) = patch.ai_provider {
        cfg.ai_provider = prov.trim().to_string();
    }
    if let Some(url) = patch.ai_base_url {
        let clean = url.trim().trim_end_matches('/').to_string();
        cfg.ai_base_url = clean.clone();
        cfg.ai_profiles.entry(cfg.ai_provider.clone()).or_default().base_url = clean;
    }
    if let Some(key) = patch.ai_api_key {
        let clean = key.trim().to_string();
        cfg.ai_api_key = clean.clone();
        cfg.ai_profiles.entry(cfg.ai_provider.clone()).or_default().api_key = clean;
    }
    if let Some(model) = patch.ai_model {
        let clean = model.trim().to_string();
        cfg.ai_model = clean.clone();
        cfg.ai_profiles.entry(cfg.ai_provider.clone()).or_default().model = clean;
    }
    if let Some(lang) = patch.target_lang {
        cfg.target_lang = lang.trim().to_string();
    }
    if let Some(w) = patch.use_whisper_fallback {
        cfg.use_whisper_fallback = w;
    }
    if let Some(wm) = patch.whisper_model {
        cfg.whisper_model = wm.trim().to_string();
    }
    if let Some(wl) = patch.whisper_language {
        let clean = wl.trim().to_string();
        cfg.whisper_language = if clean.is_empty() {
            default_whisper_language()
        } else {
            clean
        };
    }
    if let Some(wm) = patch.whisper_mode {
        let clean = wm.trim().to_string();
        cfg.whisper_mode = if clean == "cloud" { "cloud".to_string() } else { "local".to_string() };
    }
    if let Some(wcp) = patch.whisper_cloud_provider {
        cfg.whisper_cloud_provider = wcp.trim().to_string();
    }
    if let Some(wcb) = patch.whisper_cloud_base_url {
        cfg.whisper_cloud_base_url = wcb.trim().trim_end_matches('/').to_string();
    }
    if let Some(wck) = patch.whisper_cloud_api_key {
        cfg.whisper_cloud_api_key = wck.trim().to_string();
    }
    if let Some(wcm) = patch.whisper_cloud_model {
        cfg.whisper_cloud_model = wcm.trim().to_string();
    }
    if let Some(pw) = patch.prefer_whisper_over_auto_cc {
        cfg.prefer_whisper_over_auto_cc = pw;
    }
    if let Some(sm) = patch.skip_music_whisper {
        cfg.skip_music_whisper = sm;
    }
    if let Some(ys) = patch.yamnet_sensitivity {
        let clean = ys.trim().to_string();
        cfg.yamnet_sensitivity = match clean.as_str() {
            "low" | "high" => clean,
            _ => "med".to_string(),
        };
    }
    if let Some(sd) = patch.use_speaker_diarization {
        cfg.use_speaker_diarization = sd;
    }
    if let Some(tm) = patch.tts_mode {
        let clean = tm.trim().to_string();
        cfg.tts_mode = if clean == "vieneu" { "vieneu".to_string() } else { "edge".to_string() };
    }
    if let Some(vm) = patch.vieneu_model {
        let clean = vm.trim().to_string();
        if !clean.is_empty() {
            cfg.vieneu_model = clean;
        }
    }
    if let Some(vv) = patch.vieneu_voice {
        let clean = vv.trim().to_string();
        if !clean.is_empty() {
            cfg.vieneu_voice = clean;
        }
    }
    if let Some(vr) = patch.vieneu_ref_audio {
        cfg.vieneu_ref_audio = vr.trim().to_string();
    }
    if let Some(cv) = patch.custom_voices {
        cfg.custom_voices = cv
            .into_iter()
            .filter(|v| !v.id.trim().is_empty() && !v.name.trim().is_empty() && !v.ref_audio.trim().is_empty())
            .collect();
    }
    if let Some(embed) = patch.embed_subtitles {
        cfg.embed_subtitles = embed;
    }
    if let Some(max_c) = patch.max_concurrent {
        cfg.max_concurrent = max_c.clamp(1, 4);
    }
    if let Some(keep) = patch.keep_local {
        cfg.keep_local = keep;
    }
    let json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(data_dir), json).map_err(|e| e.to_string())?;
    Ok(cfg)
}
