mod config;
mod jobs;
mod rclone_stream;
mod server;
mod tools;
mod translate;
mod tts;

use config::{load_config, save_config, AppConfig, ConfigPatch};
use jobs::{EnqueueRequest, Job, JobManager, SavedIdsCache};
use rclone_stream::RcloneServeManager;
use serde::Serialize;
use server::{is_valid_youtube_url, start_server, ServerState};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent, State, WindowEvent};
use tools::{
    check_tools, check_whisper_status, connect_google_drive, create_remote_dir,
    download_whisper_model, install_missing_tools, install_whisper_cli, list_remote_dirs,
    list_remotes, remove_whisper_cli, remove_whisper_model, ToolsStatus, WhisperStatus,
};
use translate::SubtitleSummary;

pub struct AppState {
    pub data_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub jobs: Arc<JobManager>,
    pub rclone_serve: Arc<RcloneServeManager>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardState {
    pub tools: ToolsStatus,
    pub whisper: WhisperStatus,
    pub remotes: Vec<String>,
    pub config: AppConfig,
    pub jobs: Vec<Job>,
    pub translations: Vec<SubtitleSummary>,
    pub saved_count: usize,
    pub last_synced_at: Option<String>,
}

#[tauri::command]
async fn get_dashboard_state(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let (tools, remotes, jobs, saved) = tokio::join!(
        check_tools(&state.bin_dir),
        list_remotes(&state.bin_dir),
        state.jobs.list(),
        state.jobs.get_saved_ids()
    );
    let whisper = check_whisper_status(&state.data_dir, &state.bin_dir);
    let translations = translate::list_cached_subtitles(&state.data_dir);
    let config = load_config(&state.data_dir);
    Ok(DashboardState {
        tools,
        whisper,
        remotes,
        config,
        jobs,
        translations,
        saved_count: saved.ids.len(),
        last_synced_at: saved.last_synced_at,
    })
}

#[tauri::command]
async fn sync_cloud_ids(state: State<'_, AppState>) -> Result<SavedIdsCache, String> {
    state.jobs.sync_from_cloud().await
}

#[tauri::command]
async fn download_missing_tools(state: State<'_, AppState>) -> Result<ToolsStatus, String> {
    install_missing_tools(&state.bin_dir).await
}

#[tauri::command]
async fn test_ai_connection(state: State<'_, AppState>) -> Result<String, String> {
    let cfg = load_config(&state.data_dir);
    translate::test_ai_connection(&cfg).await
}

#[tauri::command]
async fn list_ai_models_cmd(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let cfg = load_config(&state.data_dir);
    translate::list_ai_models(&cfg).await
}

#[tauri::command]
async fn install_whisper_cli_cmd(state: State<'_, AppState>) -> Result<WhisperStatus, String> {
    install_whisper_cli(&state.bin_dir).await?;
    Ok(check_whisper_status(&state.data_dir, &state.bin_dir))
}

#[tauri::command]
async fn remove_whisper_cli_cmd(state: State<'_, AppState>) -> Result<WhisperStatus, String> {
    remove_whisper_cli(&state.bin_dir).await?;
    Ok(check_whisper_status(&state.data_dir, &state.bin_dir))
}

#[tauri::command]
async fn download_whisper_model_cmd(
    state: State<'_, AppState>,
    model_name: String,
) -> Result<WhisperStatus, String> {
    download_whisper_model(&state.data_dir, &model_name).await?;
    Ok(check_whisper_status(&state.data_dir, &state.bin_dir))
}

#[tauri::command]
async fn remove_whisper_model_cmd(
    state: State<'_, AppState>,
    model_name: String,
) -> Result<WhisperStatus, String> {
    remove_whisper_model(&state.data_dir, &model_name)?;
    Ok(check_whisper_status(&state.data_dir, &state.bin_dir))
}

#[tauri::command]
async fn open_extension_folder(app: tauri::AppHandle) -> Result<(), String> {
    let mut candidates = vec![];
    if let Ok(res_dir) = app.path().resource_dir() {
        candidates.push(res_dir.join("extension"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("extension"));
        candidates.push(cwd.join("../../extension"));
    }
    let ext_dir = candidates
        .into_iter()
        .find(|p| p.join("manifest.json").is_file())
        .ok_or_else(|| "Bundled extension folder not found".to_string())?;

    let status = if cfg!(target_os = "macos") {
        std::process::Command::new("open").args(["-R", &ext_dir.to_string_lossy()]).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", ext_dir.display()))
            .status()
    } else {
        let parent = ext_dir.parent().unwrap_or(&ext_dir);
        std::process::Command::new("xdg-open").arg(parent).status()
    }
    .map_err(|e| format!("Failed to open folder: {e}"))?;

    if !status.success() {
        return Err("Could not open extension folder".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn open_chrome_extensions() -> Result<(), String> {
    let res = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .args(["-a", "Google Chrome", "chrome://extensions"])
            .status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "chrome", "chrome://extensions"])
            .status()
    } else {
        std::process::Command::new("google-chrome")
            .arg("chrome://extensions")
            .status()
    };
    match res {
        Ok(s) if s.success() => Ok(()),
        _ => Err("Could not launch Chrome. Please open chrome://extensions manually.".to_string()),
    }
}

#[tauri::command]
async fn save_app_config(state: State<'_, AppState>, patch: ConfigPatch) -> Result<AppConfig, String> {
    save_config(&state.data_dir, patch)
}

#[tauri::command]
async fn connect_gdrive(state: State<'_, AppState>, remote_name: Option<String>) -> Result<Vec<String>, String> {
    let name = remote_name.unwrap_or_else(|| "gdrive".to_string());
    connect_google_drive(&state.bin_dir, &name).await?;
    Ok(list_remotes(&state.bin_dir).await)
}

#[tauri::command]
async fn list_dirs(state: State<'_, AppState>, path: String) -> Result<Vec<String>, String> {
    list_remote_dirs(&state.bin_dir, &path).await
}

#[tauri::command]
async fn create_dir(state: State<'_, AppState>, path: String) -> Result<(), String> {
    create_remote_dir(&state.bin_dir, &path).await
}

#[tauri::command]
async fn cancel_job(state: State<'_, AppState>, id: String) -> Result<Option<Job>, String> {
    Ok(state.jobs.cancel(&id).await)
}

#[tauri::command]
async fn delete_job(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    Ok(state.jobs.remove(&id).await)
}

#[tauri::command]
async fn retry_job(state: State<'_, AppState>, id: String) -> Result<Option<Job>, String> {
    Ok(state.jobs.retry(&id).await)
}

#[tauri::command]
async fn export_subtitle_srt(
    state: State<'_, AppState>,
    file_name: String,
    mode: Option<String>,
) -> Result<String, String> {
    translate::export_cached_srt(&state.data_dir, &file_name, mode.as_deref())
}

#[tauri::command]
async fn delete_subtitle_cache(state: State<'_, AppState>, file_name: String) -> Result<(), String> {
    translate::delete_cached_subtitle(&state.data_dir, &file_name)
}

#[tauri::command]
async fn backup_subtitles_cloud(state: State<'_, AppState>) -> Result<usize, String> {
    let cfg = load_config(&state.data_dir);
    translate::backup_subtitles_to_cloud(&state.data_dir, &state.bin_dir, &cfg.rclone_remote).await
}

#[tauri::command]
async fn restore_subtitles_cloud(state: State<'_, AppState>) -> Result<usize, String> {
    let cfg = load_config(&state.data_dir);
    translate::restore_subtitles_from_cloud(&state.data_dir, &state.bin_dir, &cfg.rclone_remote).await
}

#[tauri::command]
async fn save_url(state: State<'_, AppState>, url: String) -> Result<Job, String> {
    let trimmed = url.trim().to_string();
    if !is_valid_youtube_url(&trimmed) {
        return Err("Please enter a valid YouTube URL (https://youtube.com/... or https://youtu.be/...)".to_string());
    }
    Ok(state
        .jobs
        .enqueue(EnqueueRequest {
            url: trimmed,
            video_id: None,
            title: None,
            channel_name: None,
            timestamp: None,
            quality: None,
        })
        .await)
}

pub fn show_main_window(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("yt-cloud-saver-data"));
            let bin_dir = data_dir.join("bin");
            let _ = std::fs::create_dir_all(&bin_dir);

            let jobs = JobManager::new(data_dir.clone(), bin_dir.clone());
            let rclone_serve = Arc::new(RcloneServeManager::new());

            let jobs_for_sync = Arc::clone(&jobs);
            tauri::async_runtime::spawn(async move {
                let _ = jobs_for_sync.sync_from_cloud().await;
            });

            let server_state = ServerState {
                data_dir: data_dir.clone(),
                bin_dir: bin_dir.clone(),
                jobs: Arc::clone(&jobs),
                rclone_serve: Arc::clone(&rclone_serve),
                app_handle: app.handle().clone(),
            };
            tauri::async_runtime::spawn(async move {
                start_server(server_state).await;
            });

            app.manage(AppState {
                data_dir,
                bin_dir,
                jobs,
                rclone_serve,
            });

            let show_i = MenuItem::with_id(app, "show", "Open YT Cloud Saver", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit YT Cloud Saver", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("YT Cloud Saver (127.0.0.1:4873)")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let visible = app
                            .get_webview_window("main")
                            .and_then(|w| w.is_visible().ok())
                            .unwrap_or(false);
                        if visible {
                            hide_main_window(app);
                        } else {
                            show_main_window(app);
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            tray_builder.build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                hide_main_window(window.app_handle());
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard_state,
            save_app_config,
            connect_gdrive,
            list_dirs,
            create_dir,
            cancel_job,
            delete_job,
            retry_job,
            export_subtitle_srt,
            delete_subtitle_cache,
            backup_subtitles_cloud,
            restore_subtitles_cloud,
            save_url,
            sync_cloud_ids,
            download_missing_tools,
            test_ai_connection,
            list_ai_models_cmd,
            install_whisper_cli_cmd,
            remove_whisper_cli_cmd,
            download_whisper_model_cmd,
            remove_whisper_model_cmd,
            open_extension_folder,
            open_chrome_extensions
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                let st = app.state::<AppState>();
                let serve = Arc::clone(&st.rclone_serve);
                tauri::async_runtime::block_on(async move {
                    serve.stop().await;
                });
            }
            #[cfg(target_os = "macos")]
            if let RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = event
            {
                show_main_window(app);
            }
        });
}
