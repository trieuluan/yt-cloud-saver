# Changelog

All notable changes to **YT Cloud Saver** will be documented in this file.

## [1.1.0] - 2026-11-25

### Added
- **In-App Tool Downloader:** Automatically detects missing `yt-dlp`, `ffmpeg`, or `rclone` and adds a 1-click **Download Missing Tools** button right inside the Desktop App (no Terminal required).
- **Bundled Chrome Extension & Quick Opener:** Bundles `extension/` inside the Desktop App and adds **1. Open Extension Folder** (reveals & highlights the `extension` folder in Finder/Explorer) + **2. Open `chrome://extensions`** inside the Extension Guide.
- **Browser Cookies Option:** Added **Browser Cookies (restricted videos)** setting (`Google Chrome`, `Brave`, `Edge`, `Safari`, `Firefox`) passing `--cookies-from-browser` to `yt-dlp` for age-restricted or account-gated videos.

## [1.0.0] - 2026-11-25

### Added
- **Chrome Extension (MV3):**
  - Native **Save to Cloud** button injected next to Like/Share on YouTube Watch & Shorts pages (closed Shadow DOM).
  - Real-time progress indicator (`Queued` → `Downloading %` → `Uploading %` → `Saved ✓`) with click-to-cancel support.
  - Toolbar Popup with server health check, `rclone` folder browser/creator, **Auto-create subfolder by channel name** toggle, quality selector (`Best MP4`, `1080p`, `720p`), and **Open App** button.
  - **Fallback Cloud Player (`player.html`):** When a previously saved YouTube video becomes unavailable/removed, automatically streams the saved copy from Google Drive directly inside the YouTube player frame.
- **Desktop Companion App (Tauri v2):**
  - Lightweight Menu Bar / System Tray companion app (`macOS`, `Windows`, `Linux`) running local HTTP server on `127.0.0.1:4873`.
  - **1-Click Connect Google Drive** (`rclone config ... --auto-confirm`), folder browser, and **↻ Sync Drive** to keep `saved_ids.json` in sync with Google Drive.
  - HTTP `Range` (`206 Partial Content`) streaming endpoint (`/stream/{video_id}`) using `rclone cat` with zero local disk caching.
  - Automatic `yt-dlp` + `ffmpeg` downloading, channel-based folder organization, and automatic temp file cleanup.
- **CI/CD & Packaging:**
  - GitHub Actions release workflow (`.github/workflows/release.yml`) for multi-OS Tauri builds and `yt-cloud-saver-extension.zip`.
