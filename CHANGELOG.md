# Changelog

All notable changes to **YT Cloud Saver** will be documented in this file.

## [1.2.50] - 2026-11-29

### Fixed
- **Episode Cast Popup Detection**:
  - Added `activeTab` permission (`manifest.json`) and `extractVideoIdFromUrl()` in `popup.js` so the popup directly detects the active YouTube / Shorts / Cloud Player video ID instead of relying solely on stored `SET_ACTIVE_VIDEO`.
  - Ensured `renderSubStyle()` triggers `renderCastSection()` when toggling Auto Multi-Voice, switching TTS engines, or loading custom VieNeu voices.
  - Added `visibilitychange` sync in `content.js` to keep `SET_ACTIVE_VIDEO` up to date when switching tabs.

## [1.2.49] - 2026-11-29

### Added
- **Episode Cast & Per-Character Voice Mapping**:
  - Added `cast` and `speakerOverrides` support in subtitle bundles (`translate.rs`) and LLM translation prompts so speakers in an episode can be identified and mapped consistently.
  - Added `POST /subtitles/{video_id}/cast` endpoint (`server.rs`) and `UPDATE_SUBTITLE_CAST` / `SET_ACTIVE_VIDEO` / `GET_ACTIVE_VIDEO` messaging (`background.js`) to persist and sync cast edits live.
  - Added Episode Cast UI in the extension popup (`popup.html`, `popup.js`) with per-speaker voice selection, character name editing, gender swap, merge-to-lead, and an auto-merge toggle for small speaker clusters.
  - Updated live dubbing voice routing (`content.js`, `player.js`) to apply `speakerOverrides` and `autoMergeSmallClusters` in real time, while remaining backward-compatible with older subtitle files.

## [1.2.48] - 2026-11-29

### Fixed
- **Apple Vision CJK Confidence Tier (`ocr/pipeline.rs` / `ocr/signs.rs`):** Lowered the OCR acceptance threshold from `0.56` (`0.66` for signs) to `0.45` (`0.48` for signs) so valid CJK dialogue and scene signs where Apple Vision `VNRecognizeTextRequest` outputs its `0.500` confidence tier (e.g. short phrases or white text on bright paper) are retained while `0.300` background table noise remains filtered out.

## [1.2.47] - 2026-11-28

### Fixed & Improved
- **OCR RGB Max-Channel & Center-Spike Pixel-Diff (`ocr/pipeline.rs`):** Replaced luma-averaged ROI diff with `max(|ΔR|, |ΔG|, |ΔB|)` + center-band spike ratio (`30%–70%` horizontal zone), preventing short 2–4 character lines (e.g. `想去哪`, `就在隔壁`) or colored/white text on bright backgrounds from being skipped on static shots.
- **Primary Subtitle Baseline Priority (`ocr/pipeline.rs`):** Ranked ROI text boxes by subtitle likelihood (centeredness, height, confidence, character length) before capping to 2 lines, preventing background document/table text inside the ROI from crowding out the main dialogue subtitle.

## [1.2.46] - 2026-11-28

### Added & Improved
- **Cross-Platform Hardsub & On-Screen Sign Video OCR (`ocr/` / `macos/vision_ocr.m`):** Added dual-stream video OCR (`Apple Vision VNRecognizeTextRequest` built-in `0 MB` on macOS; `PP-OCRv4 ONNX` ~15.2 MB modular 1-click download via shared `ort` on Windows/Linux). Automatically probes 16:9 & 9:16 Shorts frames (`Auto: OCR → Whisper`), skips static frames via Pixel-Diff (`MAD`), filters watermarks, combines with `WeSpeaker + F0 + YAMNet` for `[Nam/Nữ · A/B]` speaker tags, and supports optional `Whisper` gap-fill for long unsubtitled speech.
- **Progressive 3-Min Fast-Start OCR & H.264 1-Pass Extraction (`ocr/pipeline.rs` / `translate.rs`):** Prioritized lightweight `H.264` (`avc1`, 360p/480p) streams, merged `4 fps` Hardsub ROI and `1 fps` Scene Signs into a single-pass `ffmpeg -filter_complex` decode, and streamed Segment 0 (first 3 minutes) straight through `WeSpeaker` + LLM translation so live subtitles appear on YouTube within seconds while remaining segments process in the background.
- **On-Screen Character Nameplates & Hardsub Cover (`extension/`):** Scanned upper-scene signs (`1 fps`), auto-sampled `fgHex`/`bgHex` colors & `serif`/`sans` styling, enriched `TranslationMemory.glossary` before dialogue translation, filtered background clutter via `[SKIP]`, and added **Cover Original Hardsub (`Off` / `Blur Glass` / `Solid Mask`)** + **Sign Badges** to YouTube & Cloud players.
- **Desktop UI & Extension UX Fixes (`desktop/ui/` / `extension/`):** Added the Hardsub OCR settings card in **AI & Translate**, upgraded **Activity** `↻ Re-run ▾` to 3 modes (`Re-translate`, `Re-OCR Hardsub & Signs`, `Force Whisper`), fixed dropdown clipping at the bottom of the Activity list via viewport-aware `position: fixed` drop-up positioning, and fixed `AI CC` mode-switching race conditions during active progressive translation.

## [1.2.45] - 2026-11-27

### Fixed & Improved
- **Google Drive Web `mov_text` Subtitle Smoothness (`translate.rs` / `jobs.rs`):** Normalized embedded MP4 subtitle cues (`normalize_cues_for_mov_text`) to eliminate overlapping timestamps, merge near-simultaneous `< 60ms` starts, extend ultra-short cues into silent pauses (`>= 580ms`) without shifting `start_ms`, and close `<= 120ms` micro-gaps that generated 2-byte empty `tx3g` clear packets. Added explicit stream mapping (`-map 0:v:0 -map 0:a? -map 1:s:0`), ISO 639-2 language metadata (`language=vie`/`eng`/`jpn`...), `default` disposition, and `-movflags +faststart`.
- **Cross-Chunk `TranslationMemory` & Re-translate Speaker Hints (`translate.rs`):** Shared `TranslationMemory` (`glossary` + `scene_note`, expanded up to 80 terms) and video `title` across all Whisper chunks, persisted them in partial `SubtitleBundle` caches for seamless resume, and reconstructed `[Nam · A]` / `[Nữ · B]` hints from `cue.speaker` during Re-translate and resume.
- **Two-Layer Context-Aware Pronoun & ASR Prompt (`translate.rs`):** Added universal language-agnostic scene-shift pronoun adaptation when acoustic speaker tags are shared across same-gender characters of different generations/roles (e.g. male lead vs father/elder/boss), while isolating Chinese ASR homophone rules (`没事吧` ↔ `没事爸`, `你` ↔ `您`) strictly to Chinese source batches.
- **Stronger Whisper Hallucination & Stutter Cleanup (`translate.rs`):** Stripped punctuation when detecting 1-character repetition loops (catching `嗯?` / `啊！` runs), collapsed 4+ character stutters (`啊啊啊啊啊啊...`), and applied cleanup during **Re-translate** on older cached subtitle bundles.

## [1.2.44] - 2026-11-27

### Fixed & Improved
- **Whisper Hallucination Loop Filter (`translate.rs`):** Dropped consecutive runs (`>= 3` cues) of digit-only or single-character Whisper silence/music hallucinations before translation while preserving genuine short dialogue, covered by unit test `test_cleanup_whisper_hallucination_runs`.
- **Modular Feature Guide & Interactive Walkthrough (`README.md` / `docs/`):** Added a complete modular feature guide, 100% real-DOM interactive walkthrough (`demo.gif` + `demo.mp4` with VieNeu-TTS voiceover & English subtitles), and updated popup footer version.

## [1.2.43] - 2026-11-27

### Fixed
- **Multi-Voice Smoothness & Caching (`extension/content.js`):** Fixed `pruneRamCacheWindow` accidentally evicting prefetched audio for secondary/opposite-gender voices on the next cue, and enabled `prepareBatchAhead` (24-cue lookahead) when `multiVoice` is active by grouping upcoming cues per resolved voice.

## [1.2.42] - 2026-11-27

### Added & Improved
- **Gemini Thinking Budget Control (UI + Backend):** Added a **Gemini Thinking** selector in **AI & Translate** (`0` = Off / cheapest default, `1024` = Light, `-1` = Model default) with automatic fallback if a Gemini model rejects `thinkingConfig`, cutting wasted thinking output tokens by ~90% on `gemini-3.8-flash`.
- **Larger Translation Batches:** Increased `TRANSLATE_BATCH_SIZE` from `50` to `80` cues per request to reduce repeated prompt token overhead.

## [1.2.41] - 2026-11-27

### Improved
- **2-Pass Anchor Clustering + AHC (`diarize.rs`):** Built clean anchor speaker clusters via Agglomerative Hierarchical Clustering on longer, cleaner utterances (`>= 1400ms`), assigned all short/noisy cues in Pass 2 with clear-F0 gender guard, locked cluster gender after `>= 5` F0 samples, smoothed isolated short `A -> X -> A` flips, and prevented YAMNet music noise from flipping `F0 > 176Hz` female clusters to Male.
- **Auto-Fetch Missing Video Titles (`translate.rs` / `lib.rs`):** Automatically resolved YouTube video titles via YouTube oEmbed when `title` is empty or matches `video_id`, including on **Re-run** (`retranslate_subtitle_cache` / `rewhisper_subtitle_cache`).

## [1.2.40] - 2026-11-27

### Improved
- **Single-Pass YAMNet & Job-Scoped ORT Session Cache:** Reused YAMNet gender frame scores from the pre-Whisper music filter in `SpeakerTracker::diarize_chunk_with_yamnet` (halving YAMNet inference per chunk) and cached `ort::Session` instances for YAMNet and WeSpeaker per transcription job so models load once and release RAM on completion.

## [1.2.39] - 2026-11-27

### Fixed
- **Cloud Whisper Diarization & YAMNet (MP3 -> WAV):** Decoded Cloud Whisper `.mp3` chunks to temporary 16kHz mono WAV (`chunk_XXX_pcm.wav`) for YAMNet music filtering and WeSpeaker speaker diarization, fixing `Ill-formed WAVE file: no RIFF tag found`.
- **Per-Cue Resilience & Diagnostics:** Skipped individual failed cues in `diarize_chunk` instead of aborting the entire chunk, and logged chunk-level diarization errors to `diarize.log`.

## [1.2.38] - 2026-11-27

### Fixed
- **Extension Manifest:** Fixed JSON syntax typo in `extension/manifest.json`.

## [1.2.37] - 2026-11-27

### Fixed
- **Shared ONNX Runtime Environment (`ort`):** Shared a single `OnceLock<Environment>` (`shared_ort_env`) across YAMNet and WeSpeaker in `yamnet.rs` and `diarize.rs`, fixing the `"only one environment is allowed per process"` error that prevented speaker diarization from running after YAMNet, and logged diarization errors instead of silently ignoring them.

## [1.2.36] - 2026-11-27

### Fixed
- **Speaker Metadata Persistence (`_raw.json`):** Ran `diarize_chunk` before saving `_raw.json` in `transcribe_audio_with_whisper` so speaker tags (`F:A`, `M:B`, ...) are preserved across resumed jobs and re-translations.
- **Multi-Voice Prefetch & Rotation:** Removed the early return in `prefetchNextCues` when `multiVoice` is enabled, and normalized female speaker pool counting (`k.includes('|F:')`) in `extension/content.js`.

## [1.2.35] - 2026-11-27

### Fixed
- **Multi-Voice Popup Polish:** Fixed `Auto Multi-Voice` checkbox alignment & label wrapping, shortened `Giọng Nữ (F)` / `Giọng Nam (M)` dropdown labels to avoid clipping, and added the secondary character rotation note.

## [1.2.34] - 2026-11-27

### Added
- **Multilingual Speaker Diarization & Cluster-Level Gender Hints:** Added `diarize.rs` using `wespeaker_en_voxceleb_resnet34_LM.onnx` (~26.5 MB via Rust `ort`), 80-bin Log-Mel Fbank (`rustfft`), `F0` pitch estimation, and YAMNet voting to group speakers (`A, B, C...`) and infer cluster gender (`Nam` / `Nữ`) for accurate pronoun translation.
- **Auto Multi-Voice Dubbing:** Added optional `speaker` metadata (`F:A`, `M:B`, ...) to `SubtitleCue` and an **Auto Multi-Voice** toggle in the Extension popup so VieNeu-TTS / Edge TTS automatically switches between Female and Male voices per character.

## [1.2.33] - 2026-11-27

### Improved
- **Smoother Dubbing Cadence & VieNeu 1.0x Rate:** Allowed dubbing audio to borrow up to `1.2s` of trailing silence before the next cue (80ms guard) so short lines aren't rushed, normalized VieNeu-TTS base playback rate to `1.0x`, and added a soft conciseness preference in translation prompts without sacrificing meaning.
- **UI Clarity:** Renamed `Offline Speech-to-Text (whisper.cpp)` to `Speech-to-Text (Whisper · Local / Cloud)` in Desktop settings.

## [1.2.32] - 2026-11-27

### Added & Improved
- **YAMNet ONNX Music & Singing Pre-Filter:** Automatically mutes background music and singing segments before Whisper transcription using `yamnet.onnx` (~15.4 MB via Rust `ort`), with 3 sensitivity presets (`Nhẹ`, `Cân bằng`, `Mạnh`) and toggle in `AI & Translate`.
- **Natural Subtitle Punctuation for Expressive TTS:** Added prompt rule to restore `?`, `!`, and `,` for questions, exclamations, and pauses (without adding trailing `.` to incomplete subtitle fragments) so both subtitles and TTS intonation sound natural.

## [1.2.31] - 2026-11-26

### Improved
- **Sticky Desktop Header & Footer:** Pinned the header, tab bar, and bottom footer (`Close window to keep running in Menu Bar` + `↻ Updates` / `Guide` / `Support`) so update controls stay always visible without scrolling.

## [1.2.30] - 2026-11-26

### Added & Fixed
- **Dub Voice Boost (Up to 200%):** Added a dedicated Web Audio `GainNode` + `DynamicsCompressorNode` boost slider (`50%–200%`, default `130%`) in the Extension popup so dubbed speech stays loud and clear over video audio, plus vertical centering for the `Original Vol` slider.
- **Whisper Glitch Deduplication & Overlap Cleanup:** Added `cleanup_whisper_cues` to merge Whisper repetition loops (overlapping or `<250ms` apart) and eliminate subtitle timestamp overlaps without touching genuine repeated speech.

## [1.2.29] - 2026-11-26

### Improved
- **Rolling Global Glossary & Context-Aware Homophone Guard:** Each translation batch now incrementally extracts and carries forward a global character/term glossary (up to 50 entries) plus a 1-line scene/pronoun note alongside the last 4 lines, preventing long-video character name drift and fixing ASR homophone misreads (e.g. `陈默` / `沉默`) strictly in person/vocative contexts without breaking normal literal lines.

## [1.2.28] - 2026-11-26

### Improved
- **Extension Popup UI/UX Redesign:** Compacted `Subtitles & Dub` into two fit-to-screen sub-tabs (`Voice Dubbing` and `Subtitles Style`), added quick `Edge AI / VieNeu / Browser` engine switcher, full-width `Dubbing Voice` picker with `▶ Test` preview, and fixed vertical overflow & long custom voice name clipping.

## [1.2.27] - 2026-11-26

### Added & Fixed
- **Offline VieNeu-TTS Integration (Installable Package & Model Variants):** Added 1-click `uv + vieneu` runtime installer, selectable `v3turbo` (48kHz) / `v3nano` (24kHz) model downloads, 25 Vietnamese regional preset voices + custom voice cloning, auto-managed local sidecar (`127.0.0.1:4874`), and Extension Dubbing voice selector.
- **Live Edge TTS Priority & Multi-Language macOS Fallback:** Live `/tts` requests now take priority over background `/tts/prepare` (plus `4`-slot semaphore and `8s` timeout / `3` retries), fixed `prefetchNextCues` skipping fast cues, and mapped macOS `say` offline fallback voices by target language (`vi`, `en`, `ja`, `ko`, `zh`, `fr`, `de`, `es`, `it`, `pt`, `ru`, `th`, `id`).

## [1.2.26] - 2026-11-26

### Fixed & Improved
- **Global Edge TTS Rate-Limit Guard & Offline macOS Fallback:** Enforced a shared 2-connection semaphore across `/tts` and `/tts/prepare` (plus smaller 8-cue initial / 12-cue rolling bursts) so Microsoft never throttles/drops WebSockets, and added an automatic macOS `say -v Linh` → MP3 offline fallback on `/tts` so Dubbing never drops a line.

## [1.2.25] - 2026-11-26

### Fixed
- **Edge TTS `unexpected-eof` Fix & RAM Pre-warm:** Removed stale WebSocket pooling in `tts.rs` (Microsoft closes Edge TTS sockets after 1 turn, causing `unexpected-eof` 502s on reuse) so every synthesis uses a clean ~1s WebSocket with auto-retry. Also pauses playback briefly during `Dub: Buffering...` to pre-cache 16 cues on disk and preload the first 6 cues into browser RAM for 0ms start.

## [1.2.24] - 2026-11-26

### Improved
- **Natural Spoken Subtitle Prompt & 12-Cue Context:** Expanded previous-context window from 6 to 12 lines and upgraded translation instructions to prioritize natural spoken idioms, strict pronoun/character-name consistency, and automatic correction of ASR/Whisper homophone typos.

## [1.2.23] - 2026-11-26

### Improved
- **Persistent Edge TTS WebSocket Pool:** Reuses up to 2 live Edge TTS WebSocket connections (240s TTL, 6s timeout, auto-retry) instead of opening a new TLS/WS handshake per line, dramatically speeding up AI Dubbing and eliminating 502 stalls.
- **Cue Duration & Word-Cluster Tightening:** Splits Whisper Cloud word lists on >1.5s internal silent gaps to snap to the actual spoken phrase and caps abnormally stretched short cues (e.g. 3-character lines spanning 15s) so subtitles and Dub never trigger early across silent pauses.

## [1.2.22] - 2026-11-26

### Fixed
- **OpenAI-Compatible `stream: false`:** Explicitly sends `"stream": false` in OpenAI-compatible `/chat/completions` requests (including `Test` connection), fixing `error decoding response body` when routing through HiveRouter / 9Router or proxies that default to SSE streaming.

## [1.2.21] - 2026-11-26

### Improved
- **Word-Level Timestamp Snapping for Whisper Cloud:** Sends `timestamp_granularities[]=segment` & `word` with `verbose_json` and snaps each subtitle cue's start/end to the actual first/last spoken word timestamps, preventing subtitles from appearing early across silent pauses.

## [1.2.20] - 2026-11-26

### Fixed & Improved
- **Reliable AI Dubbing Playback:** Added 6s WebSocket timeout + automatic retry in `synthesize_edge_tts`, parallel 4-cue prefetching, audio metadata pre-warming for instant playback, seek-aware batch pre-buffering (`preparedFromIdx`/`preparedUpToIdx` reset), and speakable-text filtering so punctuation-only cues never interrupt active speech.

## [1.2.19] - 2026-11-26

### Improved
- **Animated Progress Bars for Cloud & Translate Jobs:** Added smooth width transitions, active shimmer sweep (`barFlow`), pulsing status indicator (`pulse-dot`), and minimum visible bar width (`8%`) while downloading/uploading to Drive or running Whisper/AI translation.

## [1.2.18] - 2026-11-26

### Improved
- **Silero VAD (`--vad`) for Local Whisper:** Automatically downloads `ggml-silero-v5.1.2.bin` (~864 KB) during `whisper-cli` installation (and on demand for existing installs) and passes `--vad -vm` to `whisper-cli`, preventing subtitles from starting early across silent pauses.

## [1.2.17] - 2026-11-26

### Fixed
- **Extension `manifest.json` Syntax Fix:** Fixed an accidental extra comma on the `"version"` line in `extension/manifest.json` that prevented Chrome from loading the unpacked extension.

## [1.2.16] - 2026-11-26

### Improved
- **Gemini `safetySettings: BLOCK_NONE`:** Configured official `safetySettings` (`BLOCK_NONE` for Harassment, Hate Speech, Sexually Explicit, and Dangerous Content) on Gemini translation requests so fictional movie/drama dialogue is not falsely blocked mid-batch.

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
