use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    pub yt_dlp: bool,
    pub ffmpeg: bool,
    pub rclone: bool,
}

#[derive(Debug, Deserialize)]
struct RcloneItem {
    #[serde(rename = "Path")]
    path: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Size")]
    size: Option<i64>,
    #[serde(rename = "IsDir")]
    is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudFileInfo {
    pub remote_path: String,
    pub size: u64,
}

fn join_remote_path(base: &str, rel: &str) -> String {
    let clean_base = base.trim_end_matches('/');
    let clean_rel = rel.trim_start_matches('/');
    if clean_rel.is_empty() {
        return clean_base.to_string();
    }
    if clean_base.ends_with(':') {
        format!("{clean_base}{clean_rel}")
    } else {
        format!("{clean_base}/{clean_rel}")
    }
}

fn extract_video_id_from_filename(name: &str) -> Option<String> {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    if !stem.ends_with(']') {
        return None;
    }
    let open_idx = stem.rfind('[')?;
    let id = &stem[open_idx + 1..stem.len() - 1];
    if (6..=20).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some(id.to_string())
    } else {
        None
    }
}

pub async fn scan_remote_videos(
    bin_dir: &Path,
    remote_path: &str,
) -> Result<std::collections::HashMap<String, CloudFileInfo>, String> {
    if !remote_path.contains(':') {
        return Err("Invalid rclone path".to_string());
    }
    let rclone = find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed".to_string())?;

    let out = Command::new(rclone)
        .args([
            "lsjson",
            "-R",
            "--files-only",
            "--no-modtime",
            "--no-mimetype",
            remote_path,
        ])
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let items: Vec<RcloneItem> = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("Failed to parse rclone output: {e}"))?;

    let mut map = std::collections::HashMap::new();
    for item in items {
        if !item.is_dir {
            if let Some(id) = extract_video_id_from_filename(&item.name) {
                let full_path = join_remote_path(remote_path, &item.path);
                let size = item.size.unwrap_or(0).max(0) as u64;
                map.insert(
                    id,
                    CloudFileInfo {
                        remote_path: full_path,
                        size,
                    },
                );
            }
        }
    }
    Ok(map)
}

pub async fn read_remote_range(
    bin_dir: &Path,
    remote_file: &str,
    offset: u64,
    count: u64,
) -> Result<Vec<u8>, String> {
    let rclone = find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed".to_string())?;

    let off_str = offset.to_string();
    let cnt_str = count.to_string();
    let out = Command::new(rclone)
        .args(["cat", remote_file, "--offset", &off_str, "--count", &cnt_str])
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await
        .map_err(|e| format!("Failed to stream from rclone: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(out.stdout)
}

pub fn build_env_path(bin_dir: &Path) -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut parts = vec![bin_dir.to_string_lossy().to_string()];
    if !cfg!(windows) {
        parts.push("/opt/homebrew/bin".to_string());
        parts.push("/usr/local/bin".to_string());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts.join(sep)
}

pub fn find_binary(name: &str, bin_dir: &Path) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    let local = bin_dir.join(&exe_name);
    if local.is_file() {
        return Some(local);
    }

    if !cfg!(windows) {
        for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
            let p = Path::new(dir).join(&exe_name);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let p = dir.join(&exe_name);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    None
}

pub async fn check_tools(bin_dir: &Path) -> ToolsStatus {
    ToolsStatus {
        yt_dlp: find_binary("yt-dlp", bin_dir).is_some(),
        ffmpeg: find_binary("ffmpeg", bin_dir).is_some(),
        rclone: find_binary("rclone", bin_dir).is_some(),
    }
}

pub async fn list_remotes(bin_dir: &Path) -> Vec<String> {
    let Some(rclone) = find_binary("rclone", bin_dir) else {
        return vec![];
    };
    let output = Command::new(rclone)
        .arg("listremotes")
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => vec![],
    }
}

pub async fn list_remote_dirs(bin_dir: &Path, remote_path: &str) -> Result<Vec<String>, String> {
    if !remote_path.contains(':') {
        return Err("Invalid rclone path (expected format like gdrive:folder)".to_string());
    }
    let rclone = find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed".to_string())?;

    let out = Command::new(rclone)
        .args(["lsjson", "--dirs-only", "--no-modtime", "--no-mimetype", remote_path])
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let items: Vec<RcloneItem> = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("Failed to parse rclone output: {e}"))?;

    let mut dirs: Vec<String> = items
        .into_iter()
        .filter(|i| i.is_dir)
        .map(|i| i.name)
        .collect();
    dirs.sort();
    Ok(dirs)
}

pub async fn create_remote_dir(bin_dir: &Path, remote_path: &str) -> Result<(), String> {
    if !remote_path.contains(':') {
        return Err("Invalid rclone path (expected format like gdrive:folder)".to_string());
    }
    let rclone = find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed".to_string())?;

    let out = Command::new(rclone)
        .args(["mkdir", remote_path])
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await
        .map_err(|e| format!("Failed to run rclone: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

pub async fn verify_remote_ready(bin_dir: &Path, remote_path: &str) -> Result<(), String> {
    let remote_name = remote_path
        .split(':')
        .next()
        .unwrap_or("")
        .trim();
    if remote_name.is_empty() || !remote_path.contains(':') {
        return Err("Invalid cloud destination. Open Cloud & Settings to pick a folder.".to_string());
    }
    let rclone = find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed".to_string())?;

    let out = Command::new(rclone)
        .args(["config", "show", remote_name])
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await
        .map_err(|e| format!("Failed to check rclone config: {e}"))?;

    if !out.status.success() {
        return Err(format!(
            "Cloud remote '{remote_name}:' is not configured. Open Cloud & Settings and click Connect."
        ));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let has_token = text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("token =") && trimmed.len() > "token =".len()
    });
    if !has_token {
        return Err(format!(
            "Google Drive ('{remote_name}:') is not signed in yet. Open Cloud & Settings and click Reconnect."
        ));
    }
    Ok(())
}

pub async fn connect_google_drive(bin_dir: &Path, remote_name: &str) -> Result<(), String> {
    let clean_name = remote_name.trim().trim_end_matches(':');
    if clean_name.is_empty() {
        return Err("Remote name cannot be empty".to_string());
    }
    let rclone = find_binary("rclone", bin_dir)
        .ok_or_else(|| "rclone is not installed. Please install rclone first.".to_string())?;

    let existing = list_remotes(bin_dir).await;
    let target = format!("{clean_name}:");
    let args: Vec<&str> = if existing.iter().any(|r| r == &target) {
        vec!["config", "reconnect", &target, "--auto-confirm"]
    } else {
        vec!["config", "create", clean_name, "drive", "--auto-confirm"]
    };

    let out = Command::new(rclone)
        .args(args)
        .env("PATH", build_env_path(bin_dir))
        .output()
        .await
        .map_err(|e| format!("Failed to launch Google Drive auth: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
}

async fn download_to_file(url: &str, dest: &Path) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent("yt-cloud-saver/1.1")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed ({url}): {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} downloading {url}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn find_file_recursive(dir: &Path, target_name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if let Some(found) = find_file_recursive(&p, target_name) {
                return Some(found);
            }
        } else if p.file_name().and_then(|n| n.to_str()) == Some(target_name) {
            return Some(p);
        }
    }
    None
}

async fn extract_binary_from_zip(zip_path: &Path, binary_name: &str, bin_dir: &Path) -> Result<(), String> {
    let extract_dir = bin_dir.join(format!("_extract_{binary_name}"));
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    let status = if cfg!(windows) {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    zip_path.display(),
                    extract_dir.display()
                ),
            ])
            .status()
            .await
    } else {
        Command::new("unzip")
            .args(["-o", &zip_path.to_string_lossy(), "-d", &extract_dir.to_string_lossy()])
            .status()
            .await
    }
    .map_err(|e| format!("Failed to extract archive: {e}"))?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&extract_dir);
        return Err(format!("Archive extraction exited with {status}"));
    }

    let exe_name = if cfg!(windows) {
        format!("{binary_name}.exe")
    } else {
        binary_name.to_string()
    };
    let found = find_file_recursive(&extract_dir, &exe_name)
        .ok_or_else(|| format!("{exe_name} not found inside downloaded archive"))?;
    let dest = bin_dir.join(&exe_name);
    std::fs::copy(&found, &dest).map_err(|e| e.to_string())?;
    make_executable(&dest);

    let _ = std::fs::remove_dir_all(&extract_dir);
    let _ = std::fs::remove_file(zip_path);
    Ok(())
}

pub async fn install_missing_tools(bin_dir: &Path) -> Result<ToolsStatus, String> {
    std::fs::create_dir_all(bin_dir).map_err(|e| e.to_string())?;

    if find_binary("yt-dlp", bin_dir).is_none() {
        let url = if cfg!(target_os = "macos") {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
        } else if cfg!(target_os = "windows") {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
        } else {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
        };
        let exe_name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
        let dest = bin_dir.join(exe_name);
        download_to_file(url, &dest).await?;
        make_executable(&dest);
    }

    if find_binary("rclone", bin_dir).is_none() {
        let url = if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "https://downloads.rclone.org/rclone-current-osx-arm64.zip"
            } else {
                "https://downloads.rclone.org/rclone-current-osx-amd64.zip"
            }
        } else if cfg!(target_os = "windows") {
            "https://downloads.rclone.org/rclone-current-windows-amd64.zip"
        } else {
            "https://downloads.rclone.org/rclone-current-linux-amd64.zip"
        };
        let zip_path = bin_dir.join("rclone.zip");
        download_to_file(url, &zip_path).await?;
        extract_binary_from_zip(&zip_path, "rclone", bin_dir).await?;
    }

    if find_binary("ffmpeg", bin_dir).is_none() {
        if cfg!(target_os = "macos") {
            let zip_path = bin_dir.join("ffmpeg.zip");
            download_to_file("https://evermeet.cx/ffmpeg/getrelease/zip", &zip_path).await?;
            extract_binary_from_zip(&zip_path, "ffmpeg", bin_dir).await?;
        } else if cfg!(target_os = "windows") {
            let zip_path = bin_dir.join("ffmpeg.zip");
            download_to_file(
                "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip",
                &zip_path,
            )
            .await?;
            extract_binary_from_zip(&zip_path, "ffmpeg", bin_dir).await?;
        } else {
            return Err("On Linux, please install ffmpeg via your package manager (e.g. sudo apt install ffmpeg).".to_string());
        }
    }

    Ok(check_tools(bin_dir).await)
}
