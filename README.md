<p align="center">
  <img src="docs/banner.png" alt="YT Cloud Saver Banner" width="100%">
</p>

<h1 align="center">YT Cloud Saver</h1>

<p align="center">
  Save YouTube videos straight to <b>Google Drive</b> (or any <code>rclone</code> cloud) in 1 click, with <b>AI Bilingual Subtitles</b>, <b>2-Pass Speaker Diarization</b>, <b>Auto Multi-Voice Offline Dubbing (VieNeu-TTS / Edge TTS)</b>, <b>Whisper (Local &amp; Cloud)</b>, and instant inline Cloud playback.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Chrome%20Extension-MV3-4285F4?logo=googlechrome&logoColor=white" alt="Chrome Extension MV3">
  <img src="https://img.shields.io/badge/Desktop%20App-Tauri%20v2-24C8DB?logo=tauri&logoColor=white" alt="Tauri v2">
  <img src="https://img.shields.io/badge/AI%20Subtitles-Gemini%20%7C%20OpenAI%20%7C%20Ollama-8B5CF6" alt="AI Subtitles">
  <img src="https://img.shields.io/badge/Audio%20AI-Whisper%20%7C%20YAMNet%20%7C%20WeSpeaker-F59E0B" alt="Audio AI">
  <img src="https://img.shields.io/badge/TTS%20Dubbing-VieNeu--TTS%20%7C%20Edge%20TTS-10B981" alt="TTS Dubbing">
  <img src="https://img.shields.io/badge/Cloud-Google%20Drive%20%7C%20rclone-34A853?logo=googledrive&logoColor=white" alt="Google Drive">
</p>

---

## Demo

<p align="center">
  <img src="docs/demo.gif" alt="YT Cloud Saver Demo" width="92%">
</p>

### YouTube Player + Extension Popup (Auto Multi-Voice &amp; VieNeu-TTS)
<p align="center">
  <img src="docs/preview-youtube.png" alt="YT Cloud Saver on YouTube" width="92%">
</p>

### Desktop App (`AI & Translate` · Whisper · YAMNet · WeSpeaker · VieNeu-TTS)
<p align="center">
  <img src="docs/preview-desktop.png" alt="YT Cloud Saver Desktop App" width="92%">
</p>

---

## Features

| Feature | Description |
| :--- | :--- |
| **1-Click Save &amp; Cloud Menu** | Adds a single **`Save to Cloud`** / **`Saved ✓ ▾`** pill next to Like &amp; Share with **Play from Cloud** (inline), **Re-save / Update**, and **Remove from Cloud** (2-step confirm). |
| **Native Player Bar (`AI CC` + `Dub`)** | Integrates **`AI CC`** (cycles `OFF → Bilingual → Translated`) and **`Dub`** directly into YouTube's bottom-right control bar (`.ytp-right-controls`), auto-hiding with the player controls. |
| **AI Subtitles + Whisper (Local &amp; Cloud)** | Translates YouTube transcripts via **Gemini**, **OpenAI / OpenRouter**, or **Ollama** with **Gemini Thinking Budget** control (`0` / `1024` / `-1`, saving ~90% output tokens). Falls back to chunked **`whisper.cpp` + Silero VAD** or **Cloud Whisper (Groq / OpenAI)** for videos without captions, with progressive loading, **`↻ Re-run ▾`**, auto-fetched video titles, and **`Export .srt ▾`** (**Translated / Original / Bilingual**). |
| **YAMNet ONNX Music &amp; Singing Filter** | Automatically detects and mutes background music and singing segments before Whisper transcription using `yamnet.onnx` with 3 sensitivity presets (**Low / Balanced / High**). |
| **2-Pass Speaker Diarization &amp; Gender Hints** | Uses **WeSpeaker ONNX** (`wespeaker_voxceleb.onnx`), **F0 pitch estimation**, and **YAMNet** voting with 2-Pass **Agglomerative Clustering (AHC)** to group speakers (`M:A`, `F:B`, `F:C`...) and pass character gender hints to the LLM for accurate Vietnamese pronouns. |
| **Auto Multi-Voice Dubbing (VieNeu-TTS &amp; Edge)** | **VieNeu-TTS** (local offline Vietnamese neural TTS with 4 Female + 4 Male voices + **Custom Voice Cloning** from 3–15s reference audio) alongside **Edge Neural TTS** and **Browser TTS**. Automatically switches and rotates voices per character (`M:A`, `F:B`, `F:C`, `M:D`...), with 24-cue lookahead prefetching, **Dub Voice Boost (`100–200%`)**, and **Smart Vocal Ducking** (borrows up to 1.2s of trailing silence). |
| **Subtitle Customizer (Popup)** | Two-tab Extension Popup (`Subtitles & Dub` + `Cloud & Jobs`) with live preview, 2-way **Display Mode** sync, size (`12–32px`), text/background colors, opacity (`0–100%`), Female/Male voice selectors, speed (`0.7x–2.0x`), original volume, and Dub Voice Boost. |
| **Fast Inline Cloud Player** | Streams saved MP4s from Google Drive right inside the YouTube player frame via `rclone serve http` range proxy (zero disk cache, 15-min idle auto-stop) with a YouTube-style control bar and `✕ YouTube` switch-back. |
| **Subtitle Backup &amp; Restore** | Automatically backs up cached subtitles to `<Remote>/_ytcs_subtitles/` after translation, with **`↑ Backup Subs`** and **`↓ Restore Subs`** buttons when Drive is connected. |
| **Auto-Organize &amp; Sync** | Organizes downloads into `<Destination>/<Channel Name>/`, supports browser cookies for restricted videos, and syncs `Saved ✓` status across devices via `↻ Sync Drive`. |

### Keyboard Shortcuts (YouTube &amp; Cloud Player)

| Shortcut | Action |
| :--- | :--- |
| `Option / Alt + C` | Toggle &amp; cycle **`AI CC`** (`OFF → Bilingual → Translated`) |
| `Option / Alt + D` | Toggle **`Dub`** (`ON / OFF`) |
| `Space / K`, `← / →`, `J / L`, `M`, `F` | Full YouTube-style playback shortcuts inside Cloud Player |

---

## Installation (For Everyone)

### Step 1: Install Desktop App &amp; Connect Google Drive
1. Download the installer for your OS from the [**Latest Release**](../../releases/latest):
   - **macOS:** `.dmg` (drag `YT Cloud Saver.app` into **Applications**)
     > **macOS Note:** Because the app is not notarized with an Apple Developer ID, macOS may show *"YT Cloud Saver is damaged and can't be opened"*. Run this once in Terminal to clear the quarantine flag:
     > ```bash
     > xattr -cr "/Applications/YT Cloud Saver.app"
     > ```
   - **Windows:** `.exe` / `.msi`
     > **Windows Note:** Because the app is not signed with a paid EV certificate, Windows SmartScreen may show *"Windows protected your PC"*. Click **More info → Run anyway** to install.
   - **Linux:** `.AppImage` / `.deb`
2. Open **YT Cloud Saver** — if `yt-dlp`, `ffmpeg`, or `rclone` are missing, click **Download Missing Tools** right inside the app (no terminal needed).
3. Go to **Cloud &amp; Settings** and click **Connect** to sign in to Google Drive in your browser.
4. *(Optional)* Go to **AI &amp; Translate** to configure **Gemini / OpenAI / Ollama**, download **Whisper**, **YAMNet**, **WeSpeaker**, or install **VieNeu-TTS** in 1 click.

### Step 2: Install the Chrome Extension
1. Click **Extension Guide** at the bottom-right of the Desktop App, then click **1. Open Extension Folder** and **2. Open `chrome://extensions`** *(or download `yt-cloud-saver-extension.zip` from the [**Latest Release**](../../releases/latest))*.
2. Enable **Developer mode** (top right) in Chrome.
3. Drag &amp; drop the highlighted `extension` folder onto the Chrome Extensions page (or click **Load unpacked**).

### Step 3: Save, Translate &amp; Dub
- Open any YouTube video and click **Save to Cloud** to archive it, or toggle **`AI CC`** / **`Dub`** directly in the YouTube player bar.
- Open the Extension Popup to switch between **VieNeu**, **Edge AI**, and **Browser TTS**, or enable **Auto Multi-Voice** for character-aware dubbing.

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

## Support

YT Cloud Saver is free and open source. If it saves you time or makes watching videos more enjoyable, you can buy me a coffee:

<a href="https://ko-fi.com/luantruong19136"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Support me on Ko-fi" height="36"></a>

**From Vietnam:** scan with MoMo or any banking app (VietQR / Napas 247).

<img src="docs/momo-qr.png" width="180" alt="MoMo / VietQR donation code">

---

## Disclaimer

This project is intended for personal archiving of content you have permission to save. It is not affiliated with, endorsed by, or sponsored by YouTube, Google LLC, or Alphabet Inc.
