<p align="center">
  <img src="docs/banner.png" alt="YT Cloud Saver Banner" width="100%">
</p>

<h1 align="center">YT Cloud Saver</h1>

<p align="center">
  Save YouTube videos straight to <b>Google Drive</b> (or any <code>rclone</code> cloud) in 1 click — auto-organized by channel, with automatic fallback playback if a video gets removed from YouTube.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Chrome%20Extension-MV3-4285F4?logo=googlechrome&logoColor=white" alt="Chrome Extension MV3">
  <img src="https://img.shields.io/badge/Desktop%20App-Tauri%20v2-24C8DB?logo=tauri&logoColor=white" alt="Tauri v2">
  <img src="https://img.shields.io/badge/Cloud-Google%20Drive%20%7C%20rclone-34A853?logo=googledrive&logoColor=white" alt="Google Drive">
  <img src="https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-111827" alt="Cross Platform">
</p>

---

## Demo

> **Tip for GitHub Release:** Drag and drop [`docs/demo.mp4`](docs/demo.mp4) directly into this section when editing on GitHub.com to render an inline video player with sound!

<p align="center">
  <img src="docs/preview-youtube.png" alt="YT Cloud Saver on YouTube" width="90%">
</p>

<p align="center">
  <img src="docs/preview-desktop.png" alt="YT Cloud Saver Desktop App" width="90%">
</p>

---

## Features

| Feature | Description |
| :--- | :--- |
| **1-Click Save on YouTube** | Adds a **Save to Cloud** button right next to Like & Share on YouTube Watch and Shorts pages. |
| **Best MP4 + Audio** | Uses `yt-dlp` + `ffmpeg` to fetch and merge the highest quality video & audio, then deletes local temp files after upload. |
| **Auto-Organize by Channel** | Optionally creates `<Destination>/<Channel Name>/` folders automatically on Google Drive without duplicates. |
| **Browse & Create Drive Folders** | Visual folder picker and folder creator inside both the Desktop App and Extension Popup. |
| **Cloud Library Sync** | `↻ Sync Drive` scans your Google Drive folder by `[videoId]` so `Saved ✓` persists across devices and stays accurate if you delete files on Drive. |
| **Fallback Cloud Player** | If a saved video is later removed or copyright-struck on YouTube, opening its URL automatically streams your saved copy from Google Drive right inside the YouTube player frame (no local download needed). |
| **Lightweight Menu Bar App** | Built with **Tauri v2** (~10 MB). Closing the window minimizes to the Menu Bar / System Tray while keeping the background service active. |

---

## Installation (For Everyone)

### Step 1: Install Prerequisites & Desktop App
1. Download the installer for your OS from the [**Latest Release**](../../releases/latest):
   - **macOS:** `.dmg`
   - **Windows:** `.exe` / `.msi`
   - **Linux:** `.AppImage` / `.deb`
2. Ensure `yt-dlp`, `ffmpeg`, and `rclone` are installed (on macOS: `brew install yt-dlp ffmpeg rclone`).
3. Open **YT Cloud Saver**, go to **Cloud & Settings**, and click **Connect** to sign in to Google Drive in your browser.

### Step 2: Install the Chrome Extension
1. Download and unzip `yt-cloud-saver-extension.zip` from the [**Latest Release**](../../releases/latest).
2. Open `chrome://extensions` in Chrome and enable **Developer mode** (top right).
3. Click **Load unpacked** and select the unzipped `extension` folder.

### Step 3: Save & Watch
- Open any YouTube video and click **Save to Cloud**.
- Track progress in the Desktop App or Extension Popup.

---

## Development

### Desktop App (Tauri v2)
```bash
cd desktop/src-tauri
cargo run
```

### Package Extension ZIP
```bash
./scripts/package.sh
```

### Standalone Node.js Server (Optional CLI Mode)
If you prefer running the companion server without the Tauri UI:
```bash
node server/index.js
```

---

## Disclaimer

This project is intended for personal archiving of content you have permission to save. It is not affiliated with, endorsed by, or sponsored by YouTube, Google LLC, or Alphabet Inc.
