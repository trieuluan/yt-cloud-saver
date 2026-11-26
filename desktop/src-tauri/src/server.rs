use crate::config::{load_config, save_config, ConfigPatch};
use crate::jobs::{EnqueueRequest, JobManager};
use crate::rclone_stream::RcloneServeManager;
use crate::tools::{check_tools, create_remote_dir, list_remote_dirs, list_remotes};
use crate::translate::load_cached_subtitles;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use url::Url;

#[derive(Clone)]
pub struct ServerState {
    pub data_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub jobs: Arc<JobManager>,
    pub rclone_serve: Arc<RcloneServeManager>,
    pub app_handle: tauri::AppHandle,
}

#[derive(Deserialize)]
struct DirQuery {
    path: Option<String>,
}

#[derive(Deserialize)]
struct MkdirBody {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslateBody {
    url: String,
    video_id: String,
    title: Option<String>,
}

pub fn is_valid_youtube_url(value: &str) -> bool {
    let Ok(u) = Url::parse(value) else {
        return false;
    };
    if u.scheme() != "https" {
        return false;
    }
    matches!(
        u.host_str(),
        Some("www.youtube.com" | "youtube.com" | "m.youtube.com" | "youtu.be")
    )
}

async fn health(State(st): State<ServerState>) -> impl IntoResponse {
    let (tools, remotes) = tokio::join!(check_tools(&st.bin_dir), list_remotes(&st.bin_dir));
    let config = load_config(&st.data_dir);
    Json(json!({
        "ok": true,
        "tools": tools,
        "remotes": remotes,
        "config": config,
    }))
}

async fn get_config(State(st): State<ServerState>) -> impl IntoResponse {
    Json(json!({ "config": load_config(&st.data_dir) }))
}

async fn post_config(
    State(st): State<ServerState>,
    Json(patch): Json<ConfigPatch>,
) -> impl IntoResponse {
    match save_config(&st.data_dir, patch) {
        Ok(config) => (StatusCode::OK, Json(json!({ "config": config }))),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": err }))),
    }
}

async fn get_dirs(
    State(st): State<ServerState>,
    Query(q): Query<DirQuery>,
) -> impl IntoResponse {
    let path = q.path.unwrap_or_default();
    match list_remote_dirs(&st.bin_dir, &path).await {
        Ok(dirs) => (StatusCode::OK, Json(json!({ "path": path, "dirs": dirs }))),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))),
    }
}

async fn post_mkdir(
    State(st): State<ServerState>,
    Json(body): Json<MkdirBody>,
) -> impl IntoResponse {
    match create_remote_dir(&st.bin_dir, &body.path).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "ok": true, "path": body.path }))),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))),
    }
}

async fn list_jobs(State(st): State<ServerState>) -> impl IntoResponse {
    Json(json!({ "jobs": st.jobs.list().await }))
}

async fn post_save(
    State(st): State<ServerState>,
    Json(body): Json<EnqueueRequest>,
) -> impl IntoResponse {
    if !is_valid_youtube_url(&body.url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Valid YouTube URL is required" })),
        );
    }
    let job = st.jobs.enqueue(body).await;
    (StatusCode::ACCEPTED, Json(json!({ "job": job })))
}

async fn get_job(State(st): State<ServerState>, Path(id): Path<String>) -> impl IntoResponse {
    match st.jobs.get(&id).await {
        Some(job) => (StatusCode::OK, Json(json!({ "job": job }))),
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "Job not found" }))),
    }
}

async fn cancel_job(State(st): State<ServerState>, Path(id): Path<String>) -> impl IntoResponse {
    match st.jobs.cancel(&id).await {
        Some(job) => (StatusCode::OK, Json(json!({ "job": job }))),
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "Job not found" }))),
    }
}

async fn delete_job(State(st): State<ServerState>, Path(id): Path<String>) -> impl IntoResponse {
    if st.jobs.remove(&id).await {
        (StatusCode::OK, Json(json!({ "ok": true })))
    } else {
        (StatusCode::NOT_FOUND, Json(json!({ "error": "Job not found" })))
    }
}

async fn show_app(State(st): State<ServerState>) -> impl IntoResponse {
    crate::show_main_window(&st.app_handle);
    Json(json!({ "ok": true }))
}

async fn post_translate(
    State(st): State<ServerState>,
    Json(body): Json<TranslateBody>,
) -> impl IntoResponse {
    if !is_valid_youtube_url(&body.url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Valid YouTube URL is required" })),
        );
    }
    let cfg = load_config(&st.data_dir);
    if let Some(bundle) = load_cached_subtitles(&st.data_dir, &body.video_id, &cfg.target_lang) {
        if !bundle.partial {
            return (StatusCode::OK, Json(json!({ "cached": true, "bundle": bundle })));
        }
    }
    let title = body.title.unwrap_or_else(|| body.video_id.clone());
    let job = st.jobs.enqueue_translate(body.url, body.video_id, title).await;
    (StatusCode::ACCEPTED, Json(json!({ "cached": false, "job": job })))
}

async fn get_subtitles(
    State(st): State<ServerState>,
    Path(video_id): Path<String>,
) -> impl IntoResponse {
    let cfg = load_config(&st.data_dir);
    match load_cached_subtitles(&st.data_dir, &video_id, &cfg.target_lang) {
        Some(bundle) => (StatusCode::OK, Json(json!(bundle))),
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "No cached subtitles" }))),
    }
}

async fn get_saved_ids(State(st): State<ServerState>) -> impl IntoResponse {
    Json(st.jobs.get_saved_ids().await)
}

async fn delete_saved_video(
    State(st): State<ServerState>,
    Path(video_id): Path<String>,
) -> impl IntoResponse {
    match st.jobs.remove_saved_video(&video_id).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "ok": true }))),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))),
    }
}

async fn post_sync(State(st): State<ServerState>) -> impl IntoResponse {
    match st.jobs.sync_from_cloud().await {
        Ok(cache) => (StatusCode::OK, Json(json!(cache))),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))),
    }
}

fn split_remote_and_rel(remote_path: &str) -> Option<(&str, &str)> {
    let idx = remote_path.find(':')?;
    let root = &remote_path[..=idx];
    let rel = remote_path[idx + 1..].trim_start_matches('/');
    Some((root, rel))
}

async fn stream_video(
    State(st): State<ServerState>,
    Path(video_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(file_info) = st.jobs.get_saved_file(&video_id).await else {
        return (StatusCode::NOT_FOUND, "Saved video not found in cloud cache").into_response();
    };

    let Some((remote_root, rel_path)) = split_remote_and_rel(&file_info.remote_path) else {
        return (StatusCode::BAD_REQUEST, "Invalid remote path").into_response();
    };

    let port = match st.rclone_serve.ensure_running(&st.bin_dir, remote_root).await {
        Ok(p) => p,
        Err(err) => return (StatusCode::BAD_GATEWAY, err).into_response(),
    };

    let mut target_url = match Url::parse(&format!("http://127.0.0.1:{port}/")) {
        Ok(u) => u,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    };
    if let Ok(mut segs) = target_url.path_segments_mut() {
        for seg in rel_path.split('/').filter(|s| !s.is_empty()) {
            segs.push(seg);
        }
    }

    let mut req = st.rclone_serve.client().get(target_url);
    if let Some(range_val) = headers.get(header::RANGE) {
        req = req.header(header::RANGE, range_val);
    }

    let upstream = match req.send().await {
        Ok(r) => r,
        Err(err) => return (StatusCode::BAD_GATEWAY, format!("Stream upstream error: {err}")).into_response(),
    };

    let status = StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::OK);
    let mut builder = Response::builder().status(status);
    for hdr in [
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
    ] {
        if let Some(val) = upstream.headers().get(&hdr) {
            builder = builder.header(hdr, val);
        }
    }
    if upstream.headers().get(header::CONTENT_TYPE).is_none() {
        builder = builder.header(header::CONTENT_TYPE, "video/mp4");
    }

    builder
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[derive(Deserialize)]
struct TtsQuery {
    text: String,
    voice: Option<String>,
    rate: Option<String>,
}

fn resolve_vieneu_voice<'a>(cfg: &'a crate::config::AppConfig, raw_vid: &'a str) -> (&'a str, &'a str) {
    let req_vid = if raw_vid.trim().is_empty() || raw_vid == "default" {
        cfg.vieneu_voice.as_str()
    } else {
        raw_vid.trim()
    };
    if let Some(cid) = req_vid.strip_prefix("custom:") {
        if let Some(found) = cfg.custom_voices.iter().find(|v| v.id == cid) {
            return ("custom", found.ref_audio.as_str());
        }
    }
    (req_vid, cfg.vieneu_ref_audio.as_str())
}

async fn get_tts(
    State(st): State<ServerState>,
    Query(q): Query<TtsQuery>,
) -> Response {
    let voice_req = q.voice.as_deref().unwrap_or("");
    let cfg = load_config(&st.data_dir);
    let res = if let Some(raw_vid) = voice_req.strip_prefix("vieneu:") {
        let (vid, ref_audio) = resolve_vieneu_voice(&cfg, raw_vid);
        match crate::vieneu::synthesize_vieneu_tts(
            &st.data_dir,
            &st.bin_dir,
            &cfg.vieneu_model,
            &q.text,
            vid,
            ref_audio,
        )
        .await
        {
            Ok(bytes) => Ok(bytes),
            Err(_) => {
                crate::tts::synthesize_edge_tts_with_fallback(
                    &st.data_dir,
                    &st.bin_dir,
                    &q.text,
                    Some("vi-VN-HoaiMyNeural"),
                    q.rate.as_deref(),
                )
                .await
            }
        }
    } else {
        match crate::tts::synthesize_edge_tts_with_fallback(
            &st.data_dir,
            &st.bin_dir,
            &q.text,
            q.voice.as_deref(),
            q.rate.as_deref(),
        )
        .await
        {
            Ok(bytes) => Ok(bytes),
            Err(err) => Err(err),
        }
    };

    match res {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "audio/mpeg")
            .header(header::CACHE_CONTROL, "public, max-age=86400")
            .body(Body::from(bytes))
            .unwrap(),
        Err(err) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": err }))).into_response(),
    }
}

#[derive(Deserialize)]
struct TtsPrepareBody {
    texts: Vec<String>,
    voice: Option<String>,
    rate: Option<String>,
}

async fn post_tts_prepare(
    State(st): State<ServerState>,
    Json(body): Json<TtsPrepareBody>,
) -> impl IntoResponse {
    let total = body.texts.len();
    let voice_req = body.voice.as_deref().unwrap_or("");
    let ready = if let Some(raw_vid) = voice_req.strip_prefix("vieneu:") {
        let cfg = load_config(&st.data_dir);
        let (vid, ref_audio) = resolve_vieneu_voice(&cfg, raw_vid);
        let mut count = 0usize;
        for text in body.texts {
            if crate::vieneu::synthesize_vieneu_tts(
                &st.data_dir,
                &st.bin_dir,
                &cfg.vieneu_model,
                &text,
                vid,
                ref_audio,
            )
            .await
            .is_ok()
            {
                count += 1;
            }
        }
        count
    } else {
        crate::tts::prepare_edge_tts_batch(
            &st.data_dir,
            body.texts,
            body.voice,
            body.rate,
        )
        .await
    };
    Json(json!({ "ready": ready, "total": total }))
}

async fn get_vieneu_status(State(st): State<ServerState>) -> impl IntoResponse {
    let status = crate::vieneu::check_vieneu_status(&st.data_dir);
    let cfg = load_config(&st.data_dir);
    Json(json!({
        "runtimeInstalled": status.runtime_installed,
        "downloadedModels": status.downloaded_models,
        "sidecarRunning": status.sidecar_running,
        "activeModel": cfg.vieneu_model,
        "defaultVoice": cfg.vieneu_voice,
        "customVoices": cfg.custom_voices,
    }))
}

pub async fn start_server(state: ServerState) {
    let cfg = load_config(&state.data_dir);
    let data_dir_for_cleanup = state.data_dir.clone();
    let temp_dir_for_cleanup = PathBuf::from(&cfg.temp_dir);
    tokio::spawn(async move {
        // Prune stale temp_dir leftovers (>24h) on boot and keep tts_cache <= 100 MB hourly
        let tmp = temp_dir_for_cleanup.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(entries) = std::fs::read_dir(&tmp) {
                let cutoff = std::time::Duration::from_secs(24 * 3600);
                let now = std::time::SystemTime::now();
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(m) = meta.modified() {
                            if now.duration_since(m).unwrap_or_default() > cutoff {
                                let p = entry.path();
                                if p.is_dir() {
                                    let _ = std::fs::remove_dir_all(p);
                                } else {
                                    let _ = std::fs::remove_file(p);
                                }
                            }
                        }
                    }
                }
            }
        })
        .await;
        loop {
            let dir = data_dir_for_cleanup.clone();
            let _ = tokio::task::spawn_blocking(move || {
                crate::tts::cleanup_tts_cache(&dir, 100 * 1024 * 1024, 3 * 24 * 3600);
            })
            .await;
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
    let addr = format!("{}:{}", cfg.host, cfg.port);

    let app = Router::new()
        .route("/health", get(health))
        .route("/config", get(get_config).post(post_config))
        .route("/remotes/dirs", get(get_dirs))
        .route("/remotes/mkdir", post(post_mkdir))
        .route("/jobs", get(list_jobs))
        .route("/save", post(post_save))
        .route("/jobs/{id}", get(get_job).delete(delete_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
        .route("/saved-ids", get(get_saved_ids))
        .route("/saved/{video_id}", axum::routing::delete(delete_saved_video))
        .route("/sync", post(post_sync))
        .route("/translate", post(post_translate))
        .route("/subtitles/{video_id}", get(get_subtitles))
        .route("/stream/{video_id}", get(stream_video))
        .route("/tts", get(get_tts))
        .route("/tts/prepare", post(post_tts_prepare))
        .route("/vieneu/status", get(get_vieneu_status))
        .route("/show", post(show_app))
        .layer(CorsLayer::permissive())
        .with_state(state);

    if let Ok(listener) = tokio::net::TcpListener::bind(&addr).await {
        let _ = axum::serve(listener, app).await;
    }
}
