use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub port: u16,
    pub host: String,
    pub rclone_remote: String,
    pub group_by_channel: bool,
    pub quality: String,
    pub max_concurrent: usize,
    pub keep_local: bool,
    pub temp_dir: String,
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
