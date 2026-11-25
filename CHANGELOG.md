# Changelog

All notable changes to **YT Cloud Saver** will be documented in this file.

## [1.2.1] - 2026-11-26

### Added
- **In-App Support & Donation Modal:** Added a lightweight `☕ Support` button in both the Desktop App footer and Extension Popup with a direct Ko-fi link and scannable MoMo / VietQR code.

## [1.2.0] - 2026-11-26

### Added
- **AI Subtitles & Whisper Transcription:**
  - In-app AI translation (`Gemini` / `OpenAI`) with YouTube transcript extraction and local `whisper.cpp` fallback (model manager + chunked 30-minute processing + progressive subtitle loading).
  - Subtitle caching with `Export .srt ▾` dropdown (**Translated**, **Original**, **Bilingual**) and Google Drive **Backup / Restore** (`_ytcs_subtitles`) + auto-backup after translation.
- **Native Edge Neural TTS Dubbing:**
  - Built-in Rust Edge TTS WebSocket client (`vi-VN-HoaiMyNeural`, `vi-VN-NamMinhNeural`, `en-US-AriaNeural`, `en-US-GuyNeural`, plus Browser TTS fallback) with `/tts` & `/tts/prepare` batch pre-buffering, sliding RAM window, and automatic 100 MB / 3-day disk cache cleanup.
  - Configurable **Dubbing Voice**, **Dubbing Speed**, and **Original Volume** in the Extension Popup.
- **Redesigned Extension & YouTube Player UX:**
  - Two-tab Extension Popup (`Subtitles & Dub` + `Cloud & Jobs`) with real-time 2-way **Display Mode** sync, subtitle size/color/background/opacity customizer, and live preview.
  - Native **AI CC** + **Dub** controls integrated into YouTube's `.ytp-right-controls` bar (with `Option/Alt + C` & `Option/Alt + D` shortcuts) and auto-positioning subtitles when controls hide.
  - Single **`Saved ✓ ▾`** action menu (**Play from Cloud** inline inside YouTube, **Re-save / Update**, **Remove from Cloud**).
- **Fast Cloud Streaming Player:**
  - Upgraded `player.html` with a YouTube-style control bar, keyboard shortcuts, AI CC + Edge TTS dubbing, and fast `rclone serve http` range streaming with zero disk caching and 15-minute idle auto-stop.

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
