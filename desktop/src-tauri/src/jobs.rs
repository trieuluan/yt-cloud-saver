use crate::config::{load_config, AppConfig};
use crate::tools::{build_env_path, find_binary, scan_remote_videos, verify_remote_ready, CloudFileInfo};
use crate::translate::{translate_video_subtitles, write_srt_file};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    #[serde(default = "default_job_kind")]
    pub kind: String, // "cloud" | "translate"
    pub url: String,
    pub video_id: Option<String>,
    pub title: String,
    pub channel_name: String,
    pub timestamp: u64,
    pub quality: String,
    pub remote: String,
    pub status: String,
    pub progress: u8,
    pub stage_text: String,
    pub file_name: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_job_kind() -> String {
    "cloud".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueRequest {
    pub url: String,
    pub video_id: Option<String>,
    pub title: Option<String>,
    pub channel_name: Option<String>,
    pub timestamp: Option<u64>,
    pub quality: Option<String>,
}

fn is_active(status: &str) -> bool {
    matches!(status, "queued" | "downloading" | "uploading")
}

fn sanitize_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches('.')
        .chars()
        .take(100)
        .collect()
}

fn join_remote(base: &str, sub: &str) -> String {
    let clean = base.trim_end_matches('/');
    if sub.is_empty() {
        return clean.to_string();
    }
    if clean.ends_with(':') {
        format!("{clean}{sub}")
    } else {
        format!("{clean}/{sub}")
    }
}

fn format_selector(quality: &str) -> &'static str {
    match quality {
        "1080" => "bv*[height<=1080][ext=mp4]+ba[ext=m4a]/bv*[height<=1080]+ba/b[height<=1080]/b",
        "720" => "bv*[height<=720][ext=mp4]+ba[ext=m4a]/bv*[height<=720]+ba/b[height<=720]/b",
        _ => "bv*[ext=mp4]+ba[ext=m4a]/bv*+ba/b",
    }
}

fn extract_ytdlp_percent(line: &str) -> Option<u8> {
    let idx = line.find("[download]")?;
    let rest = &line[idx + 10..];
    let pct_idx = rest.find('%')?;
    let num_str = rest[..pct_idx].trim().split_whitespace().last()?;
    let val: f64 = num_str.parse().ok()?;
    Some(val.round().clamp(0.0, 99.0) as u8)
}

fn extract_rclone_percent(line: &str) -> Option<u8> {
    let pct_idx = line.find('%')?;
    let prefix = line[..pct_idx].trim();
    let num_str = prefix
        .rsplit(|c: char| !c.is_ascii_digit())
        .next()?;
    if num_str.is_empty() {
        return None;
    }
    let val: u8 = num_str.parse().ok()?;
    Some(val.min(99))
}

struct InnerState {
    jobs: Vec<Job>,
    cancels: HashMap<String, oneshot::Sender<()>>,
    saved_ids: HashSet<String>,
    saved_files: HashMap<String, CloudFileInfo>,
    last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SavedIdsCache {
    pub ids: Vec<String>,
    #[serde(default)]
    pub files: HashMap<String, CloudFileInfo>,
    pub last_synced_at: Option<String>,
}

pub struct JobManager {
    data_dir: PathBuf,
    bin_dir: PathBuf,
    inner: Mutex<InnerState>,
}

impl JobManager {
    pub fn new(data_dir: PathBuf, bin_dir: PathBuf) -> Arc<Self> {
        let jobs = Self::load_history(&data_dir);
        let cache = Self::load_saved_cache(&data_dir);
        Arc::new(Self {
            data_dir,
            bin_dir,
            inner: Mutex::new(InnerState {
                jobs,
                cancels: HashMap::new(),
                saved_ids: cache.ids.into_iter().collect(),
                saved_files: cache.files,
                last_synced_at: cache.last_synced_at,
            }),
        })
    }

    fn jobs_path(data_dir: &Path) -> PathBuf {
        data_dir.join("jobs.json")
    }

    fn saved_cache_path(data_dir: &Path) -> PathBuf {
        data_dir.join("saved_ids.json")
    }

    fn load_saved_cache(data_dir: &Path) -> SavedIdsCache {
        let Ok(raw) = fs::read_to_string(Self::saved_cache_path(data_dir)) else {
            return SavedIdsCache::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    fn persist_saved_cache_locked(
        &self,
        ids: &HashSet<String>,
        files: &HashMap<String, CloudFileInfo>,
        last_synced_at: &Option<String>,
    ) {
        let _ = fs::create_dir_all(&self.data_dir);
        let mut sorted: Vec<String> = ids.iter().cloned().collect();
        sorted.sort();
        let cache = SavedIdsCache {
            ids: sorted,
            files: files.clone(),
            last_synced_at: last_synced_at.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&cache) {
            let _ = fs::write(Self::saved_cache_path(&self.data_dir), json);
        }
    }

    pub async fn get_saved_ids(&self) -> SavedIdsCache {
        let state = self.inner.lock().await;
        let mut ids: Vec<String> = state.saved_ids.iter().cloned().collect();
        ids.sort();
        SavedIdsCache {
            ids,
            files: state.saved_files.clone(),
            last_synced_at: state.last_synced_at.clone(),
        }
    }

    pub async fn get_saved_file(&self, video_id: &str) -> Option<CloudFileInfo> {
        self.inner.lock().await.saved_files.get(video_id).cloned()
    }

    pub async fn remove_saved_video(&self, video_id: &str) -> Result<(), String> {
        let file_info = self.get_saved_file(video_id).await;
        if let Some(info) = file_info {
            let rclone_bin = find_binary("rclone", &self.bin_dir)
                .ok_or_else(|| "rclone binary not found".to_string())?;
            let out = Command::new(rclone_bin)
                .env("PATH", build_env_path(&self.bin_dir))
                .args(["deletefile", &info.remote_path])
                .output()
                .await
                .map_err(|e| format!("Failed to run rclone deletefile: {e}"))?;
            if !out.status.success() {
                let err_txt = String::from_utf8_lossy(&out.stderr);
                if !err_txt.to_lowercase().contains("not found") {
                    return Err(format!("rclone deletefile failed: {}", err_txt.trim()));
                }
            }
        }

        let mut state = self.inner.lock().await;
        state.saved_ids.remove(video_id);
        state.saved_files.remove(video_id);
        for job in state.jobs.iter_mut() {
            if job.video_id.as_deref() == Some(video_id)
                && job.kind == "cloud"
                && job.status == "done"
            {
                job.status = "deleted".to_string();
            }
        }
        state.jobs.retain(|j| j.status != "deleted");
        self.persist_saved_cache_locked(
            &state.saved_ids,
            &state.saved_files,
            &state.last_synced_at,
        );
        self.persist_locked(&state.jobs);
        Ok(())
    }

    pub async fn sync_from_cloud(&self) -> Result<SavedIdsCache, String> {
        let cfg = load_config(&self.data_dir);
        verify_remote_ready(&self.bin_dir, &cfg.rclone_remote).await?;
        let scanned = scan_remote_videos(&self.bin_dir, &cfg.rclone_remote).await?;
        let now = Some(Utc::now().to_rfc3339());

        let mut state = self.inner.lock().await;
        state.saved_ids = scanned.keys().cloned().collect();
        state.saved_files = scanned;
        state.last_synced_at = now.clone();
        self.persist_saved_cache_locked(&state.saved_ids, &state.saved_files, &state.last_synced_at);

        let mut ids: Vec<String> = state.saved_ids.iter().cloned().collect();
        ids.sort();
        Ok(SavedIdsCache {
            ids,
            files: state.saved_files.clone(),
            last_synced_at: now,
        })
    }

    fn load_history(data_dir: &Path) -> Vec<Job> {
        let Ok(raw) = fs::read_to_string(Self::jobs_path(data_dir)) else {
            return vec![];
        };
        let Ok(mut list) = serde_json::from_str::<Vec<Job>>(&raw) else {
            return vec![];
        };
        let now = Utc::now().to_rfc3339();
        for job in &mut list {
            if is_active(&job.status) {
                job.status = "error".to_string();
                job.error = Some("App restarted while job was active".to_string());
                job.updated_at = now.clone();
            }
        }
        list
    }

    fn persist_locked(&self, jobs: &[Job]) {
        let _ = fs::create_dir_all(&self.data_dir);
        let slice = if jobs.len() > MAX_HISTORY {
            &jobs[..MAX_HISTORY]
        } else {
            jobs
        };
        if let Ok(json) = serde_json::to_string_pretty(slice) {
            let _ = fs::write(Self::jobs_path(&self.data_dir), json);
        }
    }

    pub async fn list(&self) -> Vec<Job> {
        self.inner.lock().await.jobs.clone()
    }

    pub async fn get(&self, id: &str) -> Option<Job> {
        self.inner
            .lock()
            .await
            .jobs
            .iter()
            .find(|j| j.id == id)
            .cloned()
    }

    pub async fn enqueue(self: &Arc<Self>, req: EnqueueRequest) -> Job {
        let config = load_config(&self.data_dir);
        let mut state = self.inner.lock().await;

        if let Some(ref vid) = req.video_id {
            if let Some(existing) = state
                .jobs
                .iter()
                .find(|j| j.video_id.as_deref() == Some(vid.as_str()) && is_active(&j.status))
            {
                return existing.clone();
            }
        }

        let channel_name = req.channel_name.unwrap_or_default();
        let channel_folder = if config.group_by_channel {
            sanitize_folder_name(&channel_name)
        } else {
            String::new()
        };
        let target_remote = join_remote(&config.rclone_remote, &channel_folder);
        let now = Utc::now().to_rfc3339();

        let job = Job {
            id: Uuid::new_v4().to_string(),
            kind: "cloud".to_string(),
            url: req.url.clone(),
            video_id: req.video_id,
            title: req.title.unwrap_or(req.url),
            channel_name,
            timestamp: req.timestamp.unwrap_or(0),
            quality: req.quality.unwrap_or_else(|| config.quality.clone()),
            remote: target_remote,
            status: "queued".to_string(),
            progress: 0,
            stage_text: "Queued".to_string(),
            file_name: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        };

        state.jobs.insert(0, job.clone());
        self.persist_locked(&state.jobs);
        drop(state);

        self.pump().await;
        job
    }

    pub async fn enqueue_translate(
        self: &Arc<Self>,
        url: String,
        video_id: String,
        title: String,
    ) -> Job {
        let config = load_config(&self.data_dir);
        let mut state = self.inner.lock().await;

        if let Some(existing) = state.jobs.iter().find(|j| {
            j.kind == "translate"
                && j.video_id.as_deref() == Some(video_id.as_str())
                && is_active(&j.status)
        }) {
            return existing.clone();
        }

        let now = Utc::now().to_rfc3339();
        let job = Job {
            id: Uuid::new_v4().to_string(),
            kind: "translate".to_string(),
            url: url.clone(),
            video_id: Some(video_id.clone()),
            title: if title.is_empty() { url.clone() } else { title.clone() },
            channel_name: String::new(),
            timestamp: 0,
            quality: config.target_lang.clone(),
            remote: config.ai_provider.clone(),
            status: "downloading".to_string(),
            progress: 5,
            stage_text: "Starting translation...".to_string(),
            file_name: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        };

        state.jobs.insert(0, job.clone());
        self.persist_locked(&state.jobs);
        drop(state);

        let mgr = Arc::clone(self);
        let job_id = job.id.clone();
        tokio::spawn(async move {
            let (tx, mut rx) = oneshot::channel::<()>();
            {
                let mut st = mgr.inner.lock().await;
                st.cancels.insert(job_id.clone(), tx);
            }

            let mgr_prog = Arc::clone(&mgr);
            let jid_prog = job_id.clone();
            let handle = tokio::runtime::Handle::current();

            let res = tokio::select! {
                _ = &mut rx => Err("Cancelled".to_string()),
                r = translate_video_subtitles(
                    &mgr.data_dir,
                    &mgr.bin_dir,
                    &config,
                    &url,
                    &video_id,
                    &title,
                    |pct, text| {
                        let m = Arc::clone(&mgr_prog);
                        let id = jid_prog.clone();
                        handle.spawn(async move {
                            m.update_job(&id, None, Some(pct), Some(text), None, None).await;
                        });
                    },
                ) => r,
            };

            {
                let mut st = mgr.inner.lock().await;
                st.cancels.remove(&job_id);
            }

            match res {
                Ok(bundle) => {
                    mgr.update_job(
                        &job_id,
                        Some("done"),
                        Some(100),
                        Some(format!("Translated {} lines ({})", bundle.cues.len(), bundle.source)),
                        None,
                        None,
                    )
                    .await;
                    let dir = mgr.data_dir.clone();
                    let bin = mgr.bin_dir.clone();
                    let remote = load_config(&dir).rclone_remote;
                    tokio::spawn(async move {
                        let _ = crate::translate::backup_subtitles_to_cloud(&dir, &bin, &remote).await;
                    });
                }
                Err(err) => {
                    if err != "Cancelled" {
                        mgr.update_job(&job_id, Some("error"), None, Some("Failed".into()), None, Some(err))
                            .await;
                    }
                }
            }
        });

        job
    }

    pub async fn cancel(self: &Arc<Self>, id: &str) -> Option<Job> {
        let mut state = self.inner.lock().await;
        let idx = state.jobs.iter().position(|j| j.id == id)?;
        if !is_active(&state.jobs[idx].status) {
            return Some(state.jobs[idx].clone());
        }
        state.jobs[idx].status = "cancelled".to_string();
        state.jobs[idx].stage_text = "Cancelled".to_string();
        state.jobs[idx].updated_at = Utc::now().to_rfc3339();
        let job = state.jobs[idx].clone();

        if let Some(tx) = state.cancels.remove(id) {
            let _ = tx.send(());
        }
        self.persist_locked(&state.jobs);
        drop(state);

        self.pump().await;
        Some(job)
    }

    pub async fn remove(self: &Arc<Self>, id: &str) -> bool {
        let _ = self.cancel(id).await;
        let mut state = self.inner.lock().await;
        let before = state.jobs.len();
        state.jobs.retain(|j| j.id != id);
        let removed = state.jobs.len() < before;
        if removed {
            self.persist_locked(&state.jobs);
        }
        removed
    }

    pub async fn retry(self: &Arc<Self>, id: &str) -> Option<Job> {
        let old = self.get(id).await?;
        self.remove(id).await;
        if old.kind == "translate" {
            let vid = old.video_id.unwrap_or_default();
            return Some(self.enqueue_translate(old.url, vid, old.title).await);
        }
        Some(
            self.enqueue(EnqueueRequest {
                url: old.url,
                video_id: old.video_id,
                title: Some(old.title),
                channel_name: if old.channel_name.is_empty() {
                    None
                } else {
                    Some(old.channel_name)
                },
                timestamp: Some(old.timestamp),
                quality: Some(old.quality),
            })
            .await,
        )
    }

    async fn update_job(&self, id: &str, status: Option<&str>, progress: Option<u8>, stage: Option<String>, file_name: Option<String>, error: Option<String>) {
        let mut state = self.inner.lock().await;
        if let Some(job) = state.jobs.iter_mut().find(|j| j.id == id) {
            if let Some(s) = status {
                job.status = s.to_string();
            }
            if let Some(p) = progress {
                job.progress = p;
            }
            if let Some(st) = stage {
                job.stage_text = st;
            }
            if let Some(f) = file_name {
                job.file_name = Some(f);
            }
            if let Some(e) = error {
                job.error = Some(e);
            }
            job.updated_at = Utc::now().to_rfc3339();
            self.persist_locked(&state.jobs);
        }
    }

    fn pump(self: &Arc<Self>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let config = load_config(&self.data_dir);
            let mut to_start = vec![];

            {
                let state = self.inner.lock().await;
                let running = state
                    .jobs
                    .iter()
                    .filter(|j| j.status == "downloading" || j.status == "uploading")
                    .count();
                let slots = config.max_concurrent.saturating_sub(running);
                if slots > 0 {
                    for job in state.jobs.iter().rev() {
                        if job.status == "queued" {
                            to_start.push(job.clone());
                            if to_start.len() >= slots {
                                break;
                            }
                        }
                    }
                }
            }

            for job in to_start {
                let mgr = Arc::clone(self);
                let cfg = config.clone();
                tokio::spawn(async move {
                    mgr.run_job(job, cfg).await;
                });
            }
        })
    }

    async fn run_job(self: Arc<Self>, job: Job, config: AppConfig) {
        let job_dir = PathBuf::from(&config.temp_dir).join(&job.id);
        let _ = fs::create_dir_all(&job_dir);

        let (tx, mut rx) = oneshot::channel::<()>();
        {
            let mut state = self.inner.lock().await;
            state.cancels.insert(job.id.clone(), tx);
        }

        self.update_job(&job.id, Some("downloading"), Some(0), Some("Starting yt-dlp...".into()), None, None)
            .await;

        let result = tokio::select! {
            _ = &mut rx => Err("Cancelled".to_string()),
            res = self.execute_pipeline(&job, &job_dir, &config) => res,
        };

        {
            let mut state = self.inner.lock().await;
            state.cancels.remove(&job.id);
        }

        match result {
            Ok(()) => {
                if let Some(ref vid) = job.video_id {
                    let mut state = self.inner.lock().await;
                    let inserted = state.saved_ids.insert(vid.clone());
                    if let Some(fname) = state
                        .jobs
                        .iter()
                        .find(|j| j.id == job.id)
                        .and_then(|j| j.file_name.clone())
                    {
                        let size = fs::metadata(job_dir.join(&fname)).map(|m| m.len()).unwrap_or(0);
                        state.saved_files.insert(
                            vid.clone(),
                            CloudFileInfo {
                                remote_path: join_remote(&job.remote, &fname),
                                size,
                            },
                        );
                    }
                    if inserted || state.saved_files.contains_key(vid) {
                        self.persist_saved_cache_locked(
                            &state.saved_ids,
                            &state.saved_files,
                            &state.last_synced_at,
                        );
                    }
                }
                self.update_job(
                    &job.id,
                    Some("done"),
                    Some(100),
                    Some(format!("Saved to {}", job.remote)),
                    None,
                    None,
                )
                .await;
                if !config.keep_local {
                    let _ = fs::remove_dir_all(&job_dir);
                }
            }
            Err(err) => {
                if err != "Cancelled" {
                    self.update_job(
                        &job.id,
                        Some("error"),
                        None,
                        Some("Failed".into()),
                        None,
                        Some(err),
                    )
                    .await;
                }
                let _ = fs::remove_dir_all(&job_dir);
            }
        }

        let mgr = Arc::clone(&self);
        tokio::spawn(async move {
            mgr.pump().await;
        });
    }

    async fn execute_pipeline(&self, job: &Job, job_dir: &Path, config: &AppConfig) -> Result<(), String> {
        verify_remote_ready(&self.bin_dir, &job.remote).await?;
        let file_path = self.download_video(job, job_dir, &config.cookies_browser).await?;

        if config.embed_subtitles {
            if let Some(ref vid) = job.video_id {
                self.update_job(
                    &job.id,
                    None,
                    Some(99),
                    Some("Translating & embedding subtitles...".into()),
                    None,
                    None,
                )
                .await;
                if let Ok(bundle) = translate_video_subtitles(
                    &self.data_dir,
                    &self.bin_dir,
                    config,
                    &job.url,
                    vid,
                    &job.title,
                    |_, _| {},
                )
                .await
                {
                    let srt_path = job_dir.join("sub.srt");
                    let subbed_path = job_dir.join(
                        file_path
                            .file_name()
                            .map(|n| format!("sub_{}", n.to_string_lossy()))
                            .unwrap_or_else(|| "sub_video.mp4".to_string()),
                    );
                    if write_srt_file(&bundle, &srt_path).is_ok() {
                        if let Some(ffmpeg) = find_binary("ffmpeg", &self.bin_dir) {
                            let status = Command::new(ffmpeg)
                                .args([
                                    "-y",
                                    "-i",
                                    &file_path.to_string_lossy(),
                                    "-i",
                                    &srt_path.to_string_lossy(),
                                    "-c",
                                    "copy",
                                    "-c:s",
                                    "mov_text",
                                    &subbed_path.to_string_lossy(),
                                ])
                                .status()
                                .await;
                            if matches!(status, Ok(s) if s.success()) && subbed_path.is_file() {
                                let _ = fs::remove_file(&file_path);
                                let _ = fs::rename(&subbed_path, &file_path);
                            }
                        }
                    }
                }
            }
        }

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "video.mp4".to_string());

        self.update_job(
            &job.id,
            Some("uploading"),
            Some(0),
            Some(format!("Uploading to {}...", job.remote)),
            Some(file_name),
            None,
        )
        .await;

        self.upload_to_cloud(job, &file_path).await
    }

    async fn download_video(&self, job: &Job, job_dir: &Path, cookies_browser: &str) -> Result<PathBuf, String> {
        let ytdlp = find_binary("yt-dlp", &self.bin_dir)
            .ok_or_else(|| "yt-dlp is not installed".to_string())?;
        let template = job_dir.join("%(title).120B [%(id)s].%(ext)s");
        let template_str = template.to_string_lossy().to_string();

        let mut args: Vec<String> = vec![
            "--newline".into(),
            "--no-playlist".into(),
            "--windows-filenames".into(),
            "-f".into(),
            format_selector(&job.quality).into(),
            "--merge-output-format".into(),
            "mp4".into(),
            "-o".into(),
            template_str,
        ];
        if !cookies_browser.is_empty() {
            args.push("--cookies-from-browser".into());
            args.push(cookies_browser.to_string());
        }
        args.push(job.url.clone());

        let mut child = Command::new(ytdlp)
            .args(&args)
            .env("PATH", build_env_path(&self.bin_dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start yt-dlp: {e}"))?;

        if let Some(stdout) = child.stdout.take() {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(pct) = extract_ytdlp_percent(&line) {
                    self.update_job(
                        &job.id,
                        None,
                        Some(pct),
                        Some(format!("Downloading {pct}%")),
                        None,
                        None,
                    )
                    .await;
                } else if line.contains("[Merger]") || line.contains("Merging formats") {
                    self.update_job(
                        &job.id,
                        None,
                        Some(99),
                        Some("Merging video & audio...".into()),
                        None,
                        None,
                    )
                    .await;
                }
            }
        }

        let out = child
            .wait_with_output()
            .await
            .map_err(|e| format!("yt-dlp failed: {e}"))?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() {
                format!("yt-dlp exited with status {}", out.status)
            } else {
                err
            });
        }

        let entries = fs::read_dir(job_dir).map_err(|e| e.to_string())?;
        let mut fallback: Option<PathBuf> = None;
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with(".part") || name.ends_with(".ytdl") {
                continue;
            }
            if name.ends_with(".mp4") {
                return Ok(p);
            }
            fallback = Some(p);
        }

        fallback.ok_or_else(|| "Download finished but output file was not found".to_string())
    }

    async fn upload_to_cloud(&self, job: &Job, file_path: &Path) -> Result<(), String> {
        let rclone = find_binary("rclone", &self.bin_dir)
            .ok_or_else(|| "rclone is not installed".to_string())?;

        let mut child = Command::new(rclone)
            .args([
                "copy",
                &file_path.to_string_lossy(),
                &job.remote,
                "--stats=1s",
                "--stats-one-line",
                "-v",
            ])
            .env("PATH", build_env_path(&self.bin_dir))
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start rclone: {e}"))?;

        let mut stderr_tail = String::new();
        if let Some(stderr) = child.stderr.take() {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                stderr_tail.push_str(&line);
                stderr_tail.push('\n');
                if stderr_tail.len() > 2000 {
                    let start = stderr_tail.len() - 1500;
                    stderr_tail = stderr_tail[start..].to_string();
                }
                if let Some(pct) = extract_rclone_percent(&line) {
                    self.update_job(
                        &job.id,
                        None,
                        Some(pct),
                        Some(format!("Uploading {pct}%")),
                        None,
                        None,
                    )
                    .await;
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("rclone failed: {e}"))?;

        if !status.success() {
            return Err(if stderr_tail.trim().is_empty() {
                format!("rclone exited with status {status}")
            } else {
                stderr_tail.trim().to_string()
            });
        }
        Ok(())
    }
}
