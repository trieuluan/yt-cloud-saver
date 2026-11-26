# Changelog

All notable changes to **YT Cloud Saver** will be documented in this file.

## [1.2.15] - 2026-11-26

### Added & Improved
- **Whisper Cloud API Support (Groq Free / OpenAI / Custom):** Added Local vs. Cloud API mode toggle in **AI & Translate**, defaulting to Groq (`whisper-large-v3-turbo`, free ~8h audio/day) with 20-minute 48kbps MP3 chunking (< 25 MB), `Test Whisper API`, and shared `Spoken Language` support.
- **Strict Untranslated/CJK Detection & 2-Pass Retry:** Automatically detects when a lightweight model copies source text or leaves CJK/Kana characters untranslated and runs up to 2 targeted retry passes in smaller sub-batches.

## [1.2.14] - 2026-11-26

### Fixed & Improved
- **Indexed JSON Translation & Auto-Retry (No Shifted or Dropped Lines):** Switched subtitle translation from plain string arrays (`BATCH_SIZE = 100`) to indexed JSON objects (`[{"i":0,"t":"..."}]`, `BATCH_SIZE = 50`) with strict length checks and automatic split/retry for any missed lines — eliminating shifted subtitles and untranslated batches on lighter models like `gemini-3.5-flash-lite`.

## [1.2.13] - 2026-11-26

### Fixed & Improved
- **Google Drive Audio Fallback for Whisper:** When `yt-dlp` cannot download audio from YouTube (e.g. restricted/blocked/deleted video), Whisper now automatically falls back to extracting 16kHz WAV chunks from the video already saved on Google Drive (`/stream/{videoId}`).
- **Real `yt-dlp` Error Reporting & Safe Re-whisper Backup:** Surfaces the actual `yt-dlp` `ERROR:` line on failure instead of a generic message, and preserves the previous subtitle cache as `.bak` during `Re-whisper + Translate` so it is automatically restored if audio extraction fails.

## [1.2.12] - 2026-11-26

### Added & Improved
- **`↻ Re-run ▾` Dropdown on Cached Subtitles:** Replaced `↻ Re-translate` with a compact `↻ Re-run ▾` dropdown on `Sub ✓` cards offering both **↻ Re-translate** (keep original text, re-translate fast) and **🎙 Re-whisper + Translate** (clear cache, re-download audio, re-run Whisper with current model/language, and re-translate).

## [1.2.11] - 2026-11-26

### Added & Fixed
- **Whisper Spoken Language Selector (`-l auto` default):** Added a Spoken Language setting (`Auto-detect` by default, or manual `zh`, `en`, `ja`, `ko`, `vi`, `es`, `fr`) in **AI & Translate**, passing `-l <lang>` to `whisper-cli` so non-English videos no longer fall back to `whisper-cli`'s default `-l en` on noisy intros/outros.

## [1.2.10] - 2026-11-26

### Fixed
- **Whisper Resume From Interrupted Chunk:** Marked partial Whisper raw transcription caches (`partial`, `completedChunks`, `totalChunks`) so resuming an interrupted job continues Whispering remaining audio chunks (and finishes any untranslated chunk tail) instead of jumping straight to translation with incomplete subtitles.

## [1.2.9] - 2026-11-26

### Fixed & Improved
- **Whisper Hallucination Loop Prevention & Auto-Repair:** Added `-mc 0` and `-et 2.4` to `whisper-cli` to break 30s-window context repetition loops, plus automatic post-chunk detection & audio-filtered (`highpass=f=120,loudnorm`, `-bs 5`) re-transcription for stuck/low-CPS segments.
- **Live App Version Badge:** Desktop App header now displays the live app version (`v1.2.9 · Background service active`) from `CARGO_PKG_VERSION`.

## [1.2.8] - 2026-11-26

### Added & Improved
- **1-Click `↻ Re-translate` on Cached Subtitles:** Added a `↻ Re-translate` button on each `Sub ✓` card in the Desktop App (with full-width title layout and clean bottom action row). Reuses existing `original` cues & timestamps without re-downloading audio or re-running Whisper.

## [1.2.7] - 2026-11-26

### Added
- **1-Click In-App Auto-Updater (`tauri-plugin-updater`):** Desktop App now automatically checks GitHub Releases (`latest.json`) on startup (plus a `↻ Updates` button in the footer) and offers an **Update & Restart** banner to download, verify, install, and relaunch seamlessly without manually opening `.dmg`/`.exe` installers.

## [1.2.6] - 2026-11-26

### Fixed
- **Restore `AI CC: OFF` State in Cycle:** Fixed `AI CC` button getting stuck cycling between `Translated` and `Bilingual` without returning to `OFF` when 2-way Popup sync updated `defaultMode`.

## [1.2.5] - 2026-11-26

### Improved
- **Seamless Dubbing Pause & Resume:** Pausing a video now pauses the dubbed audio in place and resumes from the exact position when playback continues, instead of restarting the current line from the beginning.

## [1.2.4] - 2026-11-26

### Improved & Fixed
- **Exact Audio-Duration Dub Alignment:** Measures actual synthesized Edge TTS MP3 `duration` via `loadedmetadata` and scales `playbackRate` so each dubbed line finishes right on cue (`endMs`) without spilling into the next subtitle.
- **Original Volume Slider during Dub:** Replaced the fixed 3-option dropdown with a live `0%–100%` slider (`5%` steps) synced in real time to both YouTube and Cloud Player.
- **Dubbing Speed `1.0x` Display Fix:** Fixed `Dubbing Speed` `<select>` appearing blank when set to `1.0x` (`"1"` vs `"1.0"`).

## [1.2.3] - 2026-11-26

### Fixed
- **Prevent 99% → 50% Upload Retry Loop on Large Files:** Configured `rclone copy` with `--drive-chunk-size=64M`, `--low-level-retries=10`, `--retries=1`, and `--timeout=5m` so large MP4 uploads commit reliably without restarting from scratch and doubling reported transfer size.

## [1.2.2] - 2026-11-26

### Fixed & Improved
- **Accurate Upload Progress & Live Size/Speed/ETA:** Fixed `rclone` progress parser jumping between `99%` and `50%` on non-transfer lines, and added a dedicated stats row below the progress bar showing transferred size (`1.42 GiB / 2.12 GiB`), upload speed, and `ETA` in both the Desktop App and Extension Popup.

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
