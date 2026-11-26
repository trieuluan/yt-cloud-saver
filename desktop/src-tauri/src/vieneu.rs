use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use uuid::Uuid;

pub const VIENEU_SIDECAR_PORT: u16 = 4874;

const VIENEU_SERVER_PY: &str = r#"import io
import os
import threading
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel
import soundfile as sf
import uvicorn
from vieneu import Vieneu

mode = os.environ.get("VIENEU_MODE", "v3turbo")
tts = Vieneu(mode=mode)
infer_lock = threading.Lock()
preset_cache = {}
for _desc, vid in tts.list_preset_voices():
    preset_cache[vid] = vid

app = FastAPI()

class SynthReq(BaseModel):
    text: str
    voice: str = "Ngọc Huyền"
    ref_audio: str = ""

@app.get("/health")
def health():
    return {"ok": True, "mode": mode}

@app.get("/voices")
def voices():
    return {"voices": [{"id": vid, "desc": desc} for desc, vid in tts.list_preset_voices()]}

@app.post("/synthesize")
def synthesize(req: SynthReq):
    text = (req.text or "").strip()
    if not text:
        raise HTTPException(status_code=400, detail="Empty text")
    with infer_lock:
        if req.voice == "custom" and req.ref_audio and os.path.isfile(req.ref_audio):
            audio = tts.infer(text=text, ref_audio=req.ref_audio, denoise=True)
        else:
            vid = req.voice if req.voice in preset_cache else "Ngọc Huyền"
            voice_data = tts.get_preset_voice(vid)
            audio = tts.infer(text=text, voice=voice_data)
    sr = 24000 if mode == "v3nano" else 48000
    buf = io.BytesIO()
    sf.write(buf, audio, sr, format="WAV")
    return Response(content=buf.getvalue(), media_type="audio/wav")

if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ.get("VIENEU_PORT", "4874")), log_level="warning")
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VieneuStatus {
    pub runtime_installed: bool,
    pub downloaded_models: Vec<String>,
    pub sidecar_running: bool,
}

struct SidecarState {
    child: Child,
    model: String,
}

static SIDECAR: OnceLock<Mutex<Option<SidecarState>>> = OnceLock::new();
static SIDECAR_START_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static LAST_VIENEU_USE_SECS: AtomicU64 = AtomicU64::new(0);
static IDLE_REAPER_STARTED: AtomicBool = AtomicBool::new(false);

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn touch_vieneu_activity() {
    LAST_VIENEU_USE_SECS.store(now_unix_secs(), Ordering::Release);
}

fn ensure_idle_reaper() {
    if IDLE_REAPER_STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        tokio::spawn(async {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let last = LAST_VIENEU_USE_SECS.load(Ordering::Acquire);
                if last > 0 && now_unix_secs().saturating_sub(last) >= 180 {
                    let is_running = sidecar_mutex()
                        .try_lock()
                        .map(|g| g.is_some())
                        .unwrap_or(false);
                    if is_running {
                        stop_vieneu_sidecar().await;
                    }
                }
            }
        });
    }
}

fn sidecar_mutex() -> &'static Mutex<Option<SidecarState>> {
    SIDECAR.get_or_init(|| Mutex::new(None))
}

fn sidecar_start_lock() -> &'static Mutex<()> {
    SIDECAR_START_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn vieneu_root_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("vieneu")
}

pub fn vieneu_venv_dir(data_dir: &Path) -> PathBuf {
    vieneu_root_dir(data_dir).join("venv")
}

pub fn vieneu_python_path(data_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        vieneu_venv_dir(data_dir).join("Scripts").join("python.exe")
    } else {
        vieneu_venv_dir(data_dir).join("bin").join("python")
    }
}

pub fn vieneu_hf_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join("vieneu")
}

fn vieneu_model_marker(data_dir: &Path, model_id: &str) -> PathBuf {
    vieneu_hf_dir(data_dir).join(format!(".installed_{model_id}"))
}

pub fn validate_vieneu_model(model_id: &str) -> Result<&'static str, String> {
    match model_id {
        "v3turbo" => Ok("v3turbo"),
        "v3nano" => Ok("v3nano"),
        _ => Err(format!("Unsupported VieNeu model: {model_id}")),
    }
}

pub fn check_vieneu_status(data_dir: &Path) -> VieneuStatus {
    let runtime_installed = vieneu_python_path(data_dir).is_file()
        && vieneu_root_dir(data_dir).join(".runtime_ready").is_file();
    let mut downloaded_models = Vec::new();
    for id in ["v3turbo", "v3nano"] {
        if vieneu_model_marker(data_dir, id).is_file() {
            downloaded_models.push(id.to_string());
        }
    }
    let sidecar_running = sidecar_mutex()
        .try_lock()
        .map(|g| g.is_some())
        .unwrap_or(false);
    VieneuStatus {
        runtime_installed,
        downloaded_models,
        sidecar_running,
    }
}

async fn ensure_uv_binary(bin_dir: &Path) -> Result<PathBuf, String> {
    if let Some(existing) = crate::tools::find_binary("uv", bin_dir) {
        return Ok(existing);
    }
    fs::create_dir_all(bin_dir).map_err(|e| e.to_string())?;
    let uv_exe = bin_dir.join(if cfg!(windows) { "uv.exe" } else { "uv" });

    #[cfg(not(windows))]
    {
        let out = Command::new("sh")
            .args([
                "-c",
                "curl -LsSf https://astral.sh/uv/install.sh | sh",
            ])
            .env("UV_INSTALL_DIR", bin_dir)
            .env("UV_NO_MODIFY_PATH", "1")
            .output()
            .await
            .map_err(|e| format!("Failed to run uv installer: {e}"))?;
        if !out.status.success() || !uv_exe.is_file() {
            return Err(format!(
                "Failed to install uv: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }

    #[cfg(windows)]
    {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "irm https://astral.sh/uv/install.ps1 | iex",
            ])
            .env("UV_INSTALL_DIR", bin_dir)
            .env("UV_NO_MODIFY_PATH", "1")
            .output()
            .await
            .map_err(|e| format!("Failed to install uv on Windows: {e}"))?;
        if !out.status.success() || !uv_exe.is_file() {
            return Err(format!(
                "Failed to install uv: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }

    Ok(uv_exe)
}

pub async fn install_vieneu_runtime(data_dir: &Path, bin_dir: &Path) -> Result<(), String> {
    let uv = ensure_uv_binary(bin_dir).await?;
    let root = vieneu_root_dir(data_dir);
    let venv = vieneu_venv_dir(data_dir);
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    let venv_out = Command::new(&uv)
        .args(["venv", "--python", "3.12"])
        .arg(&venv)
        .output()
        .await
        .map_err(|e| format!("Failed to create Python venv: {e}"))?;
    if !venv_out.status.success() {
        return Err(format!(
            "uv venv failed: {}",
            String::from_utf8_lossy(&venv_out.stderr).trim()
        ));
    }

    let py = vieneu_python_path(data_dir);
    let pip_out = Command::new(&uv)
        .args(["pip", "install", "--python"])
        .arg(&py)
        .arg("vieneu")
        .output()
        .await
        .map_err(|e| format!("Failed to install vieneu package: {e}"))?;
    if !pip_out.status.success() {
        return Err(format!(
            "uv pip install vieneu failed: {}",
            String::from_utf8_lossy(&pip_out.stderr).trim()
        ));
    }

    fs::write(root.join("server.py"), VIENEU_SERVER_PY).map_err(|e| e.to_string())?;
    fs::write(root.join(".runtime_ready"), "ok").map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn stop_vieneu_sidecar() {
    let mut guard = sidecar_mutex().lock().await;
    if let Some(mut st) = guard.take() {
        let _ = st.child.kill().await;
        let _ = st.child.wait().await;
    }
}

pub async fn remove_vieneu_runtime(data_dir: &Path) -> Result<(), String> {
    stop_vieneu_sidecar().await;
    let root = vieneu_root_dir(data_dir);
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn download_vieneu_model(data_dir: &Path, model_id: &str) -> Result<(), String> {
    let mode = validate_vieneu_model(model_id)?;
    let py = vieneu_python_path(data_dir);
    if !py.is_file() {
        return Err("VieNeu-TTS runtime is not installed yet".to_string());
    }
    let hf_dir = vieneu_hf_dir(data_dir);
    fs::create_dir_all(&hf_dir).map_err(|e| e.to_string())?;

    let script = format!("from vieneu import Vieneu; t = Vieneu(mode='{mode}'); t.close()");
    let out = Command::new(&py)
        .args(["-c", &script])
        .env("HF_HOME", &hf_dir)
        .output()
        .await
        .map_err(|e| format!("Failed to run VieNeu model download: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Failed to download VieNeu {mode}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    fs::write(vieneu_model_marker(data_dir, mode), "ok").map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_vieneu_model(data_dir: &Path, model_id: &str) -> Result<(), String> {
    let mode = validate_vieneu_model(model_id)?;
    {
        let should_stop = {
            let guard = sidecar_mutex().lock().await;
            guard.as_ref().map(|s| s.model == mode).unwrap_or(false)
        };
        if should_stop {
            stop_vieneu_sidecar().await;
        }
    }
    let marker = vieneu_model_marker(data_dir, mode);
    if marker.is_file() {
        fs::remove_file(&marker).map_err(|e| e.to_string())?;
    }
    let any_left = ["v3turbo", "v3nano"]
        .iter()
        .any(|id| vieneu_model_marker(data_dir, id).is_file());
    if !any_left {
        let hf_dir = vieneu_hf_dir(data_dir);
        if hf_dir.exists() {
            let _ = fs::remove_dir_all(&hf_dir);
        }
    }
    Ok(())
}

async fn ensure_sidecar_running(data_dir: &Path, model_id: &str) -> Result<(), String> {
    let mode = validate_vieneu_model(model_id)?;
    let _start_guard = sidecar_start_lock().lock().await;

    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| e.to_string())?;

    {
        let mut guard = sidecar_mutex().lock().await;
        if let Some(st) = guard.as_mut() {
            if st.model == mode && matches!(st.child.try_wait(), Ok(None)) {
                if let Ok(resp) = client
                    .get(format!("http://127.0.0.1:{VIENEU_SIDECAR_PORT}/health"))
                    .send()
                    .await
                {
                    if resp.status().is_success() {
                        return Ok(());
                    }
                }
            }
            if let Some(mut old) = guard.take() {
                let _ = old.child.kill().await;
                let _ = old.child.wait().await;
            }
        }
    }

    let py = vieneu_python_path(data_dir);
    if !py.is_file() {
        return Err("VieNeu-TTS runtime not installed".to_string());
    }
    let server_py = vieneu_root_dir(data_dir).join("server.py");
    fs::write(&server_py, VIENEU_SERVER_PY).map_err(|e| e.to_string())?;

    let hf_dir = vieneu_hf_dir(data_dir);
    let mut child = Command::new(&py)
        .arg(&server_py)
        .env("HF_HOME", &hf_dir)
        .env("VIENEU_MODE", mode)
        .env("VIENEU_PORT", VIENEU_SIDECAR_PORT.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start VieNeu sidecar: {e}"))?;

    let health_url = format!("http://127.0.0.1:{VIENEU_SIDECAR_PORT}/health");
    for _ in 0..90 {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("VieNeu sidecar exited early with {status}"));
        }
        if let Ok(resp) = client.get(&health_url).send().await {
            if resp.status().is_success() {
                let mut guard = sidecar_mutex().lock().await;
                *guard = Some(SidecarState {
                    child,
                    model: mode.to_string(),
                });
                touch_vieneu_activity();
                ensure_idle_reaper();
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let _ = child.kill().await;
    Err("VieNeu sidecar timed out starting up".to_string())
}

fn cache_key(model_id: &str, voice_id: &str, ref_audio: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    let voice_part = if voice_id == "custom" {
        format!("custom:{ref_audio}")
    } else {
        voice_id.to_string()
    };
    hasher.update(format!("vieneu|{model_id}|{voice_part}|{text}").as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub async fn synthesize_vieneu_tts(
    data_dir: &Path,
    bin_dir: &Path,
    model_id: &str,
    text: &str,
    voice_id: &str,
    ref_audio: &str,
) -> Result<Vec<u8>, String> {
    let clean_text = text.trim();
    if clean_text.is_empty() {
        return Err("Empty text".to_string());
    }
    let mode = validate_vieneu_model(model_id)?;
    if !vieneu_python_path(data_dir).is_file() || !vieneu_model_marker(data_dir, mode).is_file() {
        return Err(format!("VieNeu model {mode} is not installed"));
    }

    let cache_dir = data_dir.join("tts_cache");
    let cache_path = cache_dir.join(format!("{}.mp3", cache_key(mode, voice_id, ref_audio, clean_text)));
    if let Ok(cached) = fs::read(&cache_path) {
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    ensure_sidecar_running(data_dir, mode).await?;
    touch_vieneu_activity();

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("http://127.0.0.1:{VIENEU_SIDECAR_PORT}/synthesize"))
        .json(&serde_json::json!({
            "text": clean_text,
            "voice": voice_id,
            "ref_audio": ref_audio,
        }))
        .send()
        .await
        .map_err(|e| format!("VieNeu sidecar request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("VieNeu sidecar returned HTTP {}", resp.status()));
    }
    let wav_bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if wav_bytes.is_empty() {
        return Err("VieNeu returned empty audio".to_string());
    }

    let ffmpeg = crate::tools::find_binary("ffmpeg", bin_dir)
        .ok_or_else(|| "ffmpeg not found for VieNeu MP3 encoding".to_string())?;
    let id = Uuid::new_v4().simple().to_string();
    let tmp_dir = std::env::temp_dir();
    let wav_path = tmp_dir.join(format!("ytcs_vn_{id}.wav"));
    let mp3_path = tmp_dir.join(format!("ytcs_vn_{id}.mp3"));
    fs::write(&wav_path, &wav_bytes).map_err(|e| e.to_string())?;

    let ff_ok = Command::new(&ffmpeg)
        .args(["-y", "-i"])
        .arg(&wav_path)
        .args(["-ar", "24000", "-ac", "1", "-b:a", "48k"])
        .arg(&mp3_path)
        .env("PATH", crate::tools::build_env_path(bin_dir))
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let _ = fs::remove_file(&wav_path);
    let mp3_bytes = if ff_ok { fs::read(&mp3_path).ok() } else { None };
    let _ = fs::remove_file(&mp3_path);

    let bytes = mp3_bytes
        .filter(|b| !b.is_empty())
        .ok_or_else(|| "Failed to encode VieNeu MP3".to_string())?;

    let _ = fs::create_dir_all(&cache_dir);
    let _ = fs::write(&cache_path, &bytes);
    Ok(bytes)
}
