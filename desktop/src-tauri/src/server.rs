use crate::config::{load_config, save_config, ConfigPatch};
use crate::jobs::{EnqueueRequest, JobManager};
use crate::tools::{check_tools, create_remote_dir, list_remote_dirs, list_remotes, read_remote_range};
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

async fn get_saved_ids(State(st): State<ServerState>) -> impl IntoResponse {
    Json(st.jobs.get_saved_ids().await)
}

async fn post_sync(State(st): State<ServerState>) -> impl IntoResponse {
    match st.jobs.sync_from_cloud().await {
        Ok(cache) => (StatusCode::OK, Json(json!(cache))),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))),
    }
}

async fn stream_video(
    State(st): State<ServerState>,
    Path(video_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(file_info) = st.jobs.get_saved_file(&video_id).await else {
        return (StatusCode::NOT_FOUND, "Saved video not found in cloud cache").into_response();
    };

    let total = file_info.size;
    if total == 0 {
        return (StatusCode::BAD_REQUEST, "Unknown file size; please run Sync Drive").into_response();
    }

    const MAX_CHUNK: u64 = 4 * 1024 * 1024;
    let range_hdr = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let (start, end) = if let Some(spec) = range_hdr.strip_prefix("bytes=") {
        let mut parts = spec.split('-');
        let s = parts.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let req_end = parts
            .next()
            .and_then(|v| if v.is_empty() { None } else { v.parse::<u64>().ok() });
        let capped_end = match req_end {
            Some(e) => e.min(total - 1).min(s + MAX_CHUNK - 1),
            None => (s + MAX_CHUNK - 1).min(total - 1),
        };
        (s, capped_end)
    } else {
        (0, (MAX_CHUNK - 1).min(total - 1))
    };

    if start >= total || start > end {
        return (
            StatusCode::RANGE_NOT_SATISFIABLE,
            [(header::CONTENT_RANGE, format!("bytes */{total}"))],
        )
            .into_response();
    }

    let count = end - start + 1;
    match read_remote_range(&st.bin_dir, &file_info.remote_path, start, count).await {
        Ok(bytes) => {
            let actual_len = bytes.len() as u64;
            let actual_end = start + actual_len.saturating_sub(1);
            Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, "video/mp4")
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, actual_len.to_string())
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{actual_end}/{total}"),
                )
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(err) => (StatusCode::BAD_GATEWAY, err).into_response(),
    }
}

pub async fn start_server(state: ServerState) {
    let cfg = load_config(&state.data_dir);
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
        .route("/sync", post(post_sync))
        .route("/stream/{video_id}", get(stream_video))
        .route("/show", post(show_app))
        .layer(CorsLayer::permissive())
        .with_state(state);

    if let Ok(listener) = tokio::net::TcpListener::bind(&addr).await {
        let _ = axum::serve(listener, app).await;
    }
}
