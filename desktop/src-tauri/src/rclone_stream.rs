use crate::tools::{build_env_path, find_binary};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const IDLE_TIMEOUT_SECS: u64 = 15 * 60;

struct ServeInner {
    child: Option<Child>,
    port: u16,
    remote_root: String,
    last_used: Instant,
    watcher_started: bool,
}

pub struct RcloneServeManager {
    inner: Arc<Mutex<ServeInner>>,
    http: reqwest::Client,
}

impl RcloneServeManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ServeInner {
                child: None,
                port: 0,
                remote_root: String::new(),
                last_used: Instant::now(),
                watcher_started: false,
            })),
            http: reqwest::Client::new(),
        }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.http
    }

    pub async fn ensure_running(&self, bin_dir: &Path, remote_root: &str) -> Result<u16, String> {
        let mut guard = self.inner.lock().await;
        let same_root = guard.remote_root == remote_root;
        if let Some(child) = guard.child.as_mut() {
            if same_root && matches!(child.try_wait(), Ok(None)) {
                guard.last_used = Instant::now();
                return Ok(guard.port);
            }
            let _ = child.kill().await;
            guard.child = None;
        }

        let rclone =
            find_binary("rclone", bin_dir).ok_or_else(|| "rclone is not installed".to_string())?;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to allocate port for rclone serve: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .port();
        drop(listener);

        let addr = format!("127.0.0.1:{port}");
        let mut child = Command::new(rclone)
            .args([
                "serve",
                "http",
                remote_root,
                "--addr",
                &addr,
                "--vfs-cache-mode",
                "off",
                "--dir-cache-time",
                "5m",
                "--no-checksum",
                "--no-modtime",
                "--read-only",
            ])
            .env("PATH", build_env_path(bin_dir))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start rclone serve http: {e}"))?;

        let mut ready = false;
        for _ in 0..45 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            if TcpStream::connect(&addr).await.is_ok() {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        if !ready {
            let _ = child.kill().await;
            return Err("rclone serve http did not become ready in time".to_string());
        }

        guard.child = Some(child);
        guard.port = port;
        guard.remote_root = remote_root.to_string();
        guard.last_used = Instant::now();

        if !guard.watcher_started {
            guard.watcher_started = true;
            let inner_ref = Arc::clone(&self.inner);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    let mut st = inner_ref.lock().await;
                    if st.child.is_some()
                        && st.last_used.elapsed() > Duration::from_secs(IDLE_TIMEOUT_SECS)
                    {
                        if let Some(mut c) = st.child.take() {
                            let _ = c.kill().await;
                        }
                        st.port = 0;
                        st.remote_root.clear();
                    }
                }
            });
        }

        Ok(port)
    }

    pub async fn stop(&self) {
        let mut guard = self.inner.lock().await;
        if let Some(mut child) = guard.child.take() {
            let _ = child.kill().await;
        }
        guard.port = 0;
        guard.remote_root.clear();
    }
}
