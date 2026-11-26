(() => {
  const HOST_ID = 'yt-cloud-saver-host';
  const PLAYER_FRAME_ID = 'yt-cloud-saver-player';
  const SUB_OVERLAY_ID = 'yt-cloud-saver-subs';
  const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);

  let currentJob = null;
  let isSavedInCloud = false;
  let pollTimer = null;
  let buttonEl = null;
  let labelEl = null;
  let progressEl = null;
  let subBtnEl = null;
  let dubBtnEl = null;

  let subBundle = null;
  let subMode = 'off'; // 'off' | 'bilingual' | 'translated'
  let dubEnabled = false;
  let lastSpokenCueIdx = -1;
  let savedVolume = null;
  let syncRaf = null;
  let subStyle = {
    defaultMode: 'bilingual',
    fontSize: 18,
    textColor: '#ffffff',
    bgColor: '#000000',
    bgOpacity: 80,
    dubVoice: 'vi-VN-HoaiMyNeural',
    dubSpeed: 1.0,
    dubVolume: 0.15,
    dubVoiceGain: 1.3,
    multiVoice: false,
    dubFemaleVoice: 'vieneu:Ngọc Huyền',
    dubMaleVoice: 'vieneu:Hải Đăng',
  };

  chrome.storage.local.get({ subStyle }, (res) => {
    if (res?.subStyle) subStyle = { ...subStyle, ...res.subStyle };
  });
  chrome.storage.onChanged.addListener((changes, area) => {
    if (area === 'local' && changes.subStyle?.newValue) {
      const prevDefault = subStyle.defaultMode;
      subStyle = { ...subStyle, ...changes.subStyle.newValue };
      if (subMode !== 'off' && prevDefault !== subStyle.defaultMode) {
        subMode = subStyle.defaultMode;
        updateSubButtons();
      }
      if (dubEnabled && savedVolume !== null) {
        const video = document.querySelector('video.html5-main-video') || document.querySelector('video');
        if (video) video.volume = Math.min(savedVolume, Number(subStyle.dubVolume ?? 0.15));
      }
    }
  });

  function hexToRgba(hex, alpha) {
    const clean = (hex || '#000000').replace('#', '');
    const r = parseInt(clean.slice(0, 2), 16) || 0;
    const g = parseInt(clean.slice(2, 4), 16) || 0;
    const b = parseInt(clean.slice(4, 6), 16) || 0;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  function sendBg(type, payload = {}) {
    return new Promise((resolve, reject) => {
      chrome.runtime.sendMessage({ type, ...payload }, (res) => {
        if (chrome.runtime.lastError) {
          return reject(new Error(chrome.runtime.lastError.message));
        }
        if (!res?.ok) {
          return reject(new Error(res?.error || 'Extension error'));
        }
        resolve(res.data);
      });
    });
  }

  function getVideoId() {
    const url = new URL(location.href);
    if (url.pathname === '/watch') {
      return url.searchParams.get('v');
    }
    const shortsMatch = url.pathname.match(/^\/shorts\/([A-Za-z0-9_-]+)/);
    return shortsMatch ? shortsMatch[1] : null;
  }

  function getCleanVideoUrl(videoId) {
    if (location.pathname.startsWith('/shorts/')) {
      return `https://www.youtube.com/shorts/${videoId}`;
    }
    return `https://www.youtube.com/watch?v=${videoId}`;
  }

  function getVideoTitle() {
    const h1 =
      document.querySelector('ytd-watch-metadata h1 yt-formatted-string') ||
      document.querySelector('h1.ytd-watch-metadata') ||
      document.querySelector('ytd-reel-video-renderer[is-active] h2');
    const text = h1?.textContent?.trim();
    if (text) return text;
    return document.title.replace(/\s*-\s*YouTube$/, '').trim();
  }

  function getChannelName() {
    const el =
      document.querySelector('ytd-watch-metadata ytd-channel-name a') ||
      document.querySelector('#owner #channel-name a') ||
      document.querySelector('ytd-reel-video-renderer[is-active] ytd-channel-name a');
    return el?.textContent?.trim() || '';
  }

  function getCurrentTimestamp() {
    const video = document.querySelector('video.html5-main-video') || document.querySelector('video');
    return video ? Math.floor(video.currentTime || 0) : 0;
  }

  const PLAYER_BAR_HOST_ID = 'ytcs-player-bar-host';
  let saveMenuEl = null;
  let removeConfirmArmed = false;

  function closeSaveMenu() {
    if (saveMenuEl) saveMenuEl.classList.remove('open');
    removeConfirmArmed = false;
    const remBtn = saveMenuEl?.querySelector('[data-act="remove"]');
    if (remBtn) remBtn.textContent = '🗑 Remove from Cloud';
  }

  function createButtonHost() {
    const host = document.createElement('div');
    host.id = HOST_ID;
    host.style.display = 'inline-flex';
    host.style.alignItems = 'center';
    host.style.marginLeft = '8px';
    host.style.position = 'relative';

    const shadow = host.attachShadow({ mode: 'closed' });
    shadow.innerHTML = `
      <style>
        :host {
          font-family: "Roboto", "Arial", sans-serif;
          position: relative;
        }
        button {
          position: relative;
          overflow: hidden;
          display: inline-flex;
          align-items: center;
          gap: 6px;
          height: 36px;
          padding: 0 14px;
          border: none;
          border-radius: 18px;
          background: rgba(255, 255, 255, 0.1);
          color: var(--yt-spec-text-primary, #f1f1f1);
          font-size: 14px;
          font-weight: 500;
          line-height: 36px;
          cursor: pointer;
          transition: background 0.15s ease, color 0.15s ease;
          white-space: nowrap;
        }
        html[dark="false"] button,
        :host(.light) button {
          background: rgba(0, 0, 0, 0.05);
          color: #0f0f0f;
        }
        button:hover {
          background: rgba(255, 255, 255, 0.2);
        }
        button[data-state="active"] {
          background: rgba(62, 166, 255, 0.18);
          color: #3ea6ff;
        }
        button[data-state="done"] {
          background: rgba(43, 166, 64, 0.2);
          color: #4ade80;
        }
        button[data-state="error"] {
          background: rgba(255, 78, 69, 0.2);
          color: #ff6b6b;
        }
        .progress {
          position: absolute;
          left: 0;
          bottom: 0;
          height: 3px;
          width: 0%;
          background: #3ea6ff;
          transition: width 0.25s ease;
        }
        svg {
          width: 18px;
          height: 18px;
          flex-shrink: 0;
          fill: currentColor;
        }
        .save-menu {
          position: absolute;
          bottom: calc(100% + 8px);
          top: auto;
          right: 0;
          width: 215px;
          background: #1b1f29;
          border: 1px solid #2e3444;
          border-radius: 12px;
          padding: 6px;
          box-shadow: 0 -12px 32px rgba(0, 0, 0, 0.55);
          display: none;
          flex-direction: column;
          gap: 2px;
          z-index: 9999;
        }
        .save-menu.open {
          display: flex;
        }
        .menu-item {
          width: 100%;
          height: auto;
          line-height: 1.3;
          padding: 8px 10px;
          border-radius: 8px;
          background: transparent;
          color: #e2e8f0;
          font-size: 12.5px;
          font-weight: 500;
          justify-content: flex-start;
        }
        .menu-item:hover {
          background: #272d3b;
        }
        .menu-item.danger {
          color: #f87171;
        }
        .menu-item.danger:hover {
          background: rgba(239, 68, 68, 0.15);
        }
        .menu-sep {
          height: 1px;
          background: #2b3140;
          margin: 3px 4px;
        }
      </style>
      <button type="button" class="main-save-btn" data-state="idle" title="Save video to Cloud via local server">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M19.35 10.04A7.49 7.49 0 0 0 12 4C9.11 4 6.6 5.64 5.35 8.04A5.994 5.994 0 0 0 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM17 13l-5 5-5-5h3V9h4v4h3z"/>
        </svg>
        <span class="label">Save to Cloud</span>
        <span class="progress"></span>
      </button>
      <div class="save-menu">
        <button type="button" class="menu-item" data-act="play">▶ Play from Cloud</button>
        <button type="button" class="menu-item" data-act="resave">↻ Re-save / Update</button>
        <div class="menu-sep"></div>
        <button type="button" class="menu-item danger" data-act="remove">🗑 Remove from Cloud</button>
      </div>
    `;

    buttonEl = shadow.querySelector('.main-save-btn');
    labelEl = shadow.querySelector('.label');
    progressEl = shadow.querySelector('.progress');
    saveMenuEl = shadow.querySelector('.save-menu');

    buttonEl.addEventListener('click', (e) => {
      e.stopPropagation();
      onButtonClick();
    });
    saveMenuEl.addEventListener('click', (e) => {
      e.stopPropagation();
      const item = e.target.closest('.menu-item');
      if (item) onSaveMenuAction(item.dataset.act);
    });
    return host;
  }

  function ensurePlayerBarControls() {
    const rightControls = document.querySelector('#movie_player .ytp-right-controls');
    if (!rightControls) return;
    const existing = document.getElementById(PLAYER_BAR_HOST_ID);
    if (existing && existing.parentElement === rightControls) return;
    if (existing) existing.remove();

    const host = document.createElement('div');
    host.id = PLAYER_BAR_HOST_ID;
    Object.assign(host.style, {
      display: 'inline-flex',
      alignItems: 'center',
      height: '100%',
      marginRight: '8px',
      verticalAlign: 'middle',
      flexShrink: '0',
    });

    const shadow = host.attachShadow({ mode: 'open' });
    shadow.innerHTML = `
      <style>
        .bar-group {
          display: inline-flex;
          align-items: center;
          gap: 4px;
          padding: 3px;
          border-radius: 999px;
          background: rgba(18, 22, 30, 0.72);
          border: 1px solid rgba(255, 255, 255, 0.16);
          backdrop-filter: blur(8px);
        }
        button {
          height: 26px;
          padding: 0 10px;
          border-radius: 999px;
          border: none;
          background: transparent;
          color: #e2e8f0;
          font-family: "Roboto", "Arial", sans-serif;
          font-size: 11.5px;
          font-weight: 600;
          display: inline-flex;
          align-items: center;
          gap: 5px;
          cursor: pointer;
          white-space: nowrap;
          transition: background 0.15s ease, color 0.15s ease;
        }
        button:hover {
          background: rgba(255, 255, 255, 0.14);
        }
        button[data-state="active"] {
          background: rgba(168, 85, 247, 0.34);
          color: #f3e8ff;
        }
        button[data-state="translated"] {
          background: rgba(59, 130, 246, 0.34);
          color: #dbeafe;
        }
        button[data-state="done"] {
          background: rgba(34, 197, 94, 0.3);
          color: #dcfce7;
        }
        button[data-state="working"] {
          background: rgba(59, 130, 246, 0.3);
          color: #dbeafe;
        }
        button[data-state="error"] {
          background: rgba(239, 68, 68, 0.3);
          color: #fecaca;
        }
        .dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: currentColor;
          opacity: 0.85;
        }
      </style>
      <div class="bar-group">
        <button type="button" class="sub-btn" data-state="idle" title="Toggle AI Subtitles (Option/Alt + C)">
          <span class="dot"></span><span class="txt">AI CC: OFF</span>
        </button>
        <button type="button" class="dub-btn" data-state="idle" title="Toggle AI Voice Dubbing (Option/Alt + D)">
          <span class="dot"></span><span class="txt">Dub: OFF</span>
        </button>
      </div>
    `;

    subBtnEl = shadow.querySelector('.sub-btn');
    dubBtnEl = shadow.querySelector('.dub-btn');
    subBtnEl.addEventListener('click', (e) => {
      e.stopPropagation();
      onSubClick();
    });
    dubBtnEl.addEventListener('click', (e) => {
      e.stopPropagation();
      onDubClick();
    });

    rightControls.insertBefore(host, rightControls.firstChild);
    updateSubButtons();
  }

  function renderJobState(job) {
    if (!buttonEl || !labelEl || !progressEl) return;

    if (!job) {
      buttonEl.dataset.state = 'idle';
      buttonEl.title = 'Save video to Cloud via local server';
      labelEl.textContent = 'Save to Cloud';
      progressEl.style.width = '0%';
      return;
    }

    if (ACTIVE_STATUSES.has(job.status)) {
      buttonEl.dataset.state = 'active';
      buttonEl.title = `${job.stageText} (Click to cancel)`;
      if (job.status === 'queued') {
        labelEl.textContent = 'Queued...';
        progressEl.style.width = '8%';
      } else if (job.status === 'downloading') {
        labelEl.textContent = `Downloading ${job.progress || 0}%`;
        progressEl.style.width = `${Math.max(5, job.progress || 0)}%`;
      } else {
        labelEl.textContent = `Uploading ${job.progress || 0}%`;
        progressEl.style.width = `${Math.max(5, job.progress || 0)}%`;
      }
      return;
    }

    if (job.status === 'done') {
      buttonEl.dataset.state = 'done';
      buttonEl.title = `Saved to ${job.remote} (Click for options)`;
      labelEl.textContent = 'Saved ✓ ▾';
      progressEl.style.width = '100%';
      return;
    }

    if (job.status === 'error') {
      buttonEl.dataset.state = 'error';
      buttonEl.title = job.error || 'Failed to save video';
      labelEl.textContent = 'Failed — Retry';
      progressEl.style.width = '0%';
      return;
    }

    buttonEl.dataset.state = 'idle';
    labelEl.textContent = 'Save to Cloud';
    progressEl.style.width = '0%';
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  function startPolling(jobId) {
    stopPolling();
    pollTimer = setInterval(async () => {
      try {
        const { job } = await sendBg('GET_JOB', { id: jobId });
        currentJob = job;
        renderJobState(job);
        if (!job || !ACTIVE_STATUSES.has(job.status)) {
          stopPolling();
        }
      } catch {
        stopPolling();
      }
    }, 1200);
  }

  async function triggerSaveVideo(videoId) {
    try {
      renderJobState({ status: 'queued', progress: 0, stageText: 'Sending...' });
      const { job } = await sendBg('SAVE_VIDEO', {
        payload: {
          url: getCleanVideoUrl(videoId),
          videoId,
          title: getVideoTitle(),
          channelName: getChannelName(),
          timestamp: getCurrentTimestamp(),
        },
      });
      currentJob = job;
      renderJobState(job);
      if (ACTIVE_STATUSES.has(job.status)) {
        startPolling(job.id);
      }
    } catch (err) {
      currentJob = null;
      renderJobState({
        status: 'error',
        error: `Local server offline or error: ${err.message}`,
      });
    }
  }

  async function onSaveMenuAction(act) {
    const videoId = getVideoId();
    if (!videoId) return;
    if (act === 'play') {
      closeSaveMenu();
      maybeMountCloudPlayer(true);
    } else if (act === 'resave') {
      closeSaveMenu();
      await triggerSaveVideo(videoId);
    } else if (act === 'remove') {
      const remBtn = saveMenuEl?.querySelector('[data-act="remove"]');
      if (!removeConfirmArmed) {
        removeConfirmArmed = true;
        if (remBtn) remBtn.textContent = '⚠️ Click again to confirm';
        return;
      }
      closeSaveMenu();
      try {
        renderJobState({ status: 'queued', progress: 0, stageText: 'Removing...' });
        await sendBg('DELETE_SAVED', { videoId });
        currentJob = null;
        isSavedInCloud = false;
        renderJobState(null);
      } catch (err) {
        renderJobState({ status: 'error', error: err.message });
      }
    }
  }

  async function onButtonClick() {
    const videoId = getVideoId();
    if (!videoId) return;

    if (currentJob && ACTIVE_STATUSES.has(currentJob.status)) {
      try {
        const { job } = await sendBg('CANCEL_JOB', { id: currentJob.id });
        currentJob = job;
        stopPolling();
        renderJobState(null);
      } catch (err) {
        renderJobState({ status: 'error', error: err.message });
      }
      return;
    }

    if (currentJob?.status === 'done' || isSavedInCloud) {
      if (saveMenuEl) {
        const willOpen = !saveMenuEl.classList.contains('open');
        closeSaveMenu();
        if (willOpen) saveMenuEl.classList.add('open');
      }
      return;
    }

    await triggerSaveVideo(videoId);
  }

  document.addEventListener('click', () => {
    closeSaveMenu();
  });

  function findActionContainer() {
    return (
      document.querySelector('ytd-watch-metadata #top-level-buttons-computed') ||
      document.querySelector('#actions-inner #top-level-buttons-computed') ||
      document.querySelector('ytd-watch-metadata #actions #menu')
    );
  }

  function removeCloudPlayer() {
    const existing = document.getElementById(PLAYER_FRAME_ID);
    if (existing) existing.remove();
  }

  function removeSubOverlay() {
    const el = document.getElementById(SUB_OVERLAY_ID);
    if (el) el.remove();
    if (syncRaf) {
      cancelAnimationFrame(syncRaf);
      syncRaf = null;
    }
    const video = document.querySelector('video.html5-main-video') || document.querySelector('video');
    dubEnabled = false;
    stopDubbingSpeech(video, true);
    ttsAudioCache.clear();
    preparedUpToIdx = -1;
    updateSubButtons();
  }

  function updateSubButtons() {
    if (subBtnEl) {
      subBtnEl.dataset.state =
        subMode === 'bilingual' ? 'active' : subMode === 'translated' ? 'translated' : 'idle';
      const label =
        subMode === 'bilingual' ? 'AI CC: Bilingual' : subMode === 'translated' ? 'AI CC: Translated' : 'AI CC: OFF';
      const txt = subBtnEl.querySelector('.txt');
      if (txt) txt.textContent = label;
    }
    if (dubBtnEl) {
      dubBtnEl.dataset.state = dubEnabled ? 'done' : 'idle';
      const txt = dubBtnEl.querySelector('.txt');
      if (txt) txt.textContent = dubEnabled ? 'Dub: ON' : 'Dub: OFF';
    }
  }

  function ensureSubOverlay() {
    let overlay = document.getElementById(SUB_OVERLAY_ID);
    if (overlay) return overlay;
    const player = document.querySelector('#movie_player') || document.querySelector('#player');
    if (!player) return null;
    if (window.getComputedStyle(player).position === 'static') {
      player.style.position = 'relative';
    }
    overlay = document.createElement('div');
    overlay.id = SUB_OVERLAY_ID;
    const syncOverlayBottom = () => {
      overlay.style.bottom = player.classList.contains('ytp-autohide') ? '22px' : '66px';
    };
    Object.assign(overlay.style, {
      position: 'absolute',
      left: '8%',
      right: '8%',
      bottom: player.classList.contains('ytp-autohide') ? '22px' : '66px',
      zIndex: '60',
      textAlign: 'center',
      pointerEvents: 'none',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: '4px',
      transition: 'bottom 0.22s ease',
    });
    if (!player._ytcsAutohideObs) {
      player._ytcsAutohideObs = new MutationObserver(syncOverlayBottom);
      player._ytcsAutohideObs.observe(player, { attributes: true, attributeFilter: ['class'] });
    }
    player.appendChild(overlay);
    return overlay;
  }

  let cachedVoices = [];
  let speakSeq = 0;
  let speakTimer = null;
  let dubAudioEl = null;
  const ttsAudioCache = new Map();

  function refreshVoices() {
    if (!window.speechSynthesis) return;
    const list = window.speechSynthesis.getVoices();
    if (list && list.length) cachedVoices = list;
  }
  if (window.speechSynthesis) {
    refreshVoices();
    window.speechSynthesis.addEventListener?.('voiceschanged', refreshVoices);
  }

  function langCodeForTarget(targetLang) {
    const map = {
      Vietnamese: 'vi-VN',
      English: 'en-US',
      Japanese: 'ja-JP',
      Korean: 'ko-KR',
      Chinese: 'zh-CN',
    };
    return map[targetLang] || 'vi-VN';
  }

  function pickVoiceForLang(langCode) {
    refreshVoices();
    if (!cachedVoices.length) return null;
    const target = langCode.toLowerCase();
    const base = target.split('-')[0];
    const matches = cachedVoices.filter((v) => {
      const vl = (v.lang || '').toLowerCase().replace('_', '-');
      return vl === target || vl.startsWith(`${base}-`) || vl === base;
    });
    if (!matches.length) return null;
    return (
      matches.find((v) => v.localService && /linh|enhanced|premium/i.test(v.name)) ||
      matches.find((v) => /linh/i.test(v.name)) ||
      matches.find((v) => /google/i.test(v.name)) ||
      matches[0]
    );
  }

  function restoreVideoVolume(video) {
    if (savedVolume !== null && video) {
      video.volume = savedVolume;
      savedVolume = null;
    }
  }

  function pauseDubbingSpeech(video) {
    if (speakTimer) {
      clearTimeout(speakTimer);
      speakTimer = null;
    }
    if (dubAudioEl && !dubAudioEl.paused) {
      try {
        dubAudioEl.pause();
      } catch {}
    }
    if (window.speechSynthesis?.speaking) {
      try {
        window.speechSynthesis.pause();
      } catch {}
    }
    restoreVideoVolume(video);
  }

  function resumeDubbingIfSameCue(video, curIdx) {
    if (!dubEnabled || !video || video.paused) return false;
    if (curIdx >= 0 && curIdx === lastSpokenCueIdx) {
      if (dubAudioEl && dubAudioEl.src && dubAudioEl.paused && !dubAudioEl.ended && dubAudioEl.currentTime > 0) {
        duckVideoVolume(video);
        dubAudioEl.play().catch(() => {});
        return true;
      }
      if (window.speechSynthesis?.paused) {
        duckVideoVolume(video);
        window.speechSynthesis.resume();
        return true;
      }
    }
    return false;
  }

  function stopDubbingSpeech(video, resetCue = false) {
    speakSeq += 1;
    if (resetCue) lastSpokenCueIdx = -1;
    if (speakTimer) {
      clearTimeout(speakTimer);
      speakTimer = null;
    }
    if (dubAudioEl) {
      try {
        dubAudioEl.onended = null;
        dubAudioEl.onerror = null;
        dubAudioEl.pause();
        dubAudioEl.removeAttribute('src');
        dubAudioEl.load();
      } catch {}
    }
    if (window.speechSynthesis) {
      try {
        window.speechSynthesis.pause();
        window.speechSynthesis.cancel();
      } catch {}
    }
    restoreVideoVolume(video);
  }

  function bindVideoDubEvents(video) {
    if (!video || video._ytcsDubBound) return;
    video._ytcsDubBound = true;
    video.addEventListener('pause', () => {
      if (dubEnabled) pauseDubbingSpeech(video);
    });
    video.addEventListener('play', () => {
      if (!dubEnabled || !subBundle?.cues?.length) return;
      const nowMs = video.currentTime * 1000;
      const curIdx = subBundle.cues.findIndex((c) => nowMs >= c.startMs && nowMs <= c.endMs);
      resumeDubbingIfSameCue(video, curIdx);
    });
    video.addEventListener('seeking', () => {
      preparedFromIdx = -1;
      preparedUpToIdx = -1;
      lastPrefetchedIdx = -1;
      if (dubEnabled) stopDubbingSpeech(video, true);
    });
    video.addEventListener('ended', () => {
      if (dubEnabled) stopDubbingSpeech(video, true);
    });
  }

  function computeDubRate(text, cue, video) {
    const playbackRate = Math.max(0.5, Number(video?.playbackRate) || 1);
    const cueMs = Math.max(400, (cue.endMs || 0) - (cue.startMs || 0));
    const availableSec = cueMs / 1000 / playbackRate;
    const clean = text.trim();
    const words = clean ? clean.split(/\s+/).length : 1;
    const estNaturalSec = Math.max(words / 2.7, clean.length / 14);
    const fitRate = estNaturalSec / Math.max(0.35, availableSec * 0.92);
    const userSpeed = Math.max(0.7, Math.min(2.0, Number(subStyle.dubSpeed) || 1.0));
    const baseRate = userSpeed * playbackRate;
    return Math.min(3.5, Math.max(baseRate, fitRate));
  }

  function hasSpeakableText(text) {
    return Boolean(text && /[\p{L}\p{N}]/u.test(text));
  }

  const speakerVoiceMap = new Map();
  function resolveCueVoice(cue) {
    const base = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
    if (base === 'browser' || !subStyle.multiVoice || !cue?.speaker) return base;
    const sp = String(cue.speaker);
    const isVieneu = base.startsWith('vieneu:');
    if (!isVieneu) {
      if (sp.startsWith('F:')) return 'vi-VN-HoaiMyNeural';
      if (sp.startsWith('M:')) return 'vi-VN-NamMinhNeural';
      return base;
    }
    const cacheKey = `${subStyle.dubFemaleVoice || ''}|${subStyle.dubMaleVoice || ''}|${sp}`;
    if (speakerVoiceMap.has(cacheKey)) return speakerVoiceMap.get(cacheKey);
    if (sp.startsWith('F:')) {
      const fPool = [
        subStyle.dubFemaleVoice || 'vieneu:Ngọc Huyền',
        'vieneu:Thùy Dung',
        'vieneu:Trúc Ly',
        'vieneu:Ngọc Trân',
      ].filter((v, idx, a) => v && a.indexOf(v) === idx);
      const count = [...speakerVoiceMap.keys()].filter((k) => k.endsWith(`|F:${k.split('|F:')[1]}`)).length;
      const chosen = fPool[count % fPool.length];
      speakerVoiceMap.set(cacheKey, chosen);
      return chosen;
    }
    if (sp.startsWith('M:')) {
      const mPool = [
        subStyle.dubMaleVoice || 'vieneu:Hải Đăng',
        'vieneu:Thiện Minh',
        'vieneu:Quang Sơn',
        'vieneu:Thái Sơn',
      ].filter((v, idx, a) => v && a.indexOf(v) === idx);
      const count = [...speakerVoiceMap.keys()].filter((k) => k.includes('|M:')).length;
      const chosen = mPool[count % mPool.length];
      speakerVoiceMap.set(cacheKey, chosen);
      return chosen;
    }
    return base;
  }

  function preloadAudioMeta(audioUrl) {
    return new Promise((resolve) => {
      const a = new Audio();
      let done = false;
      const finish = () => {
        if (done) return;
        done = true;
        resolve({
          audioUrl,
          duration: Number.isFinite(a.duration) && a.duration > 0 ? a.duration : 0,
        });
      };
      a.addEventListener('loadedmetadata', finish, { once: true });
      a.addEventListener('error', finish, { once: true });
      a.preload = 'metadata';
      a.src = audioUrl;
      setTimeout(finish, 180);
    });
  }

  function fetchEdgeTtsAudio(text, voice, rateStr) {
    const key = `${voice}|${rateStr}|${text}`;
    if (ttsAudioCache.has(key)) return ttsAudioCache.get(key);
    const runFetch = async () => {
      try {
        const res = await sendBg('SYNTH_TTS', { text, voice, rate: rateStr });
        if (!res?.audioUrl) throw new Error('Empty Edge TTS audio');
        return await preloadAudioMeta(res.audioUrl);
      } catch (firstErr) {
        await new Promise((r) => setTimeout(r, 150));
        const res2 = await sendBg('SYNTH_TTS', { text, voice, rate: rateStr });
        if (!res2?.audioUrl) throw firstErr;
        return await preloadAudioMeta(res2.audioUrl);
      }
    };
    const promise = runFetch();
    ttsAudioCache.set(key, promise);
    promise.catch(() => ttsAudioCache.delete(key));
    return promise;
  }

  function pruneRamCacheWindow(currentIdx, voice, rateStr) {
    if (!subBundle?.cues?.length || currentIdx < 0) return;
    const minKeep = Math.max(0, currentIdx - 5);
    const maxKeep = Math.min(subBundle.cues.length - 1, currentIdx + 25);
    const keepKeys = new Set();
    for (let i = minKeep; i <= maxKeep; i += 1) {
      const t = subBundle.cues[i]?.translated?.trim();
      if (t) keepKeys.add(`${voice}|${rateStr}|${t}`);
    }
    for (const key of ttsAudioCache.keys()) {
      if (!keepKeys.has(key)) {
        ttsAudioCache.delete(key);
      }
    }
  }

  let preparedFromIdx = -1;
  let preparedUpToIdx = -1;
  let prepareBatchInFlight = false;
  async function prepareBatchAhead(startIdx, count = 24) {
    const voice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
    if (voice === 'browser' || subStyle.multiVoice || !subBundle?.cues?.length) return;
    if (prepareBatchInFlight) return;
    if (preparedFromIdx >= 0 && startIdx >= preparedFromIdx && startIdx + 6 < preparedUpToIdx) return;

    const from = Math.max(0, startIdx);
    const to = Math.min(subBundle.cues.length, from + count);
    const texts = [];
    for (let i = from; i < to; i += 1) {
      const t = subBundle.cues[i]?.translated?.trim();
      if (hasSpeakableText(t)) texts.push(t);
    }
    if (!texts.length) return;

    prepareBatchInFlight = true;
    try {
      await sendBg('PREPARE_TTS', { texts, voice, rate: '+15%' }).catch(() => {});
      preparedFromIdx = preparedFromIdx >= 0 && startIdx >= preparedFromIdx ? preparedFromIdx : from;
      preparedUpToIdx = Math.max(preparedUpToIdx, to);
    } finally {
      prepareBatchInFlight = false;
    }
  }

  let lastPrefetchedIdx = -1;
  async function prefetchNextCues(idx) {
    const voice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
    if (voice === 'browser' || subStyle.multiVoice || !subBundle?.cues?.length) return;
    if (idx === lastPrefetchedIdx) return;
    lastPrefetchedIdx = idx;
    pruneRamCacheWindow(idx, voice, '+15%');
    prepareBatchAhead(idx + 1, 12);
    const tasks = [];
    for (let offset = 1; offset <= 4; offset += 1) {
      const nextCue = subBundle.cues[idx + offset];
      const nextText = nextCue?.translated?.trim();
      if (hasSpeakableText(nextText)) {
        const cueVoice = resolveCueVoice(nextCue);
        tasks.push(fetchEdgeTtsAudio(nextText, cueVoice, '+15%').catch(() => {}));
      }
    }
    if (tasks.length) await Promise.all(tasks);
  }

  function duckVideoVolume(video) {
    if (!video) return;
    if (savedVolume === null) savedVolume = video.volume;
    video.volume = Math.min(savedVolume, Number(subStyle.dubVolume ?? 0.15));
  }

  function speakCueBrowser(text, cue, video, mySeq) {
    if (!window.speechSynthesis) return;
    if (window.speechSynthesis.speaking || window.speechSynthesis.pending) {
      window.speechSynthesis.cancel();
    }
    const langCode = langCodeForTarget(subBundle?.targetLang);
    const voice = pickVoiceForLang(langCode);
    const utter = new SpeechSynthesisUtterance(text);
    utter.lang = langCode;
    if (voice) utter.voice = voice;
    const rawRate = computeDubRate(text, cue, video);
    utter.rate = voice && !voice.localService ? Math.min(2.0, rawRate) : rawRate;

    const finish = () => {
      if (mySeq !== speakSeq) return;
      restoreVideoVolume(video);
    };
    utter.onend = finish;
    utter.onerror = finish;

    speakTimer = setTimeout(() => {
      speakTimer = null;
      if (mySeq !== speakSeq || !dubEnabled || video.paused) return;
      duckVideoVolume(video);
      window.speechSynthesis.speak(utter);
    }, 25);
  }

  async function speakCue(cue, video, idx) {
    if (!cue?.translated) return;
    const text = cue.translated.trim();
    if (!hasSpeakableText(text)) return;

    const mySeq = ++speakSeq;
    if (speakTimer) {
      clearTimeout(speakTimer);
      speakTimer = null;
    }
    if (dubAudioEl) {
      try {
        dubAudioEl.pause();
      } catch {}
    }

    const voice = resolveCueVoice(cue);
    if (voice === 'browser') {
      if (dubBtnEl) dubBtnEl.title = 'AI Voice Dubbing (Browser TTS)';
      speakCueBrowser(text, cue, video, mySeq);
      return;
    }

    if (dubBtnEl) dubBtnEl.title = `Edge AI Dubbing (${voice})`;
    prefetchNextCues(idx);

    try {
      const { audioUrl, duration } = await fetchEdgeTtsAudio(text, voice, '+15%');
      if (mySeq !== speakSeq || !dubEnabled || video.paused) {
        if (!dubAudioEl || dubAudioEl.paused) restoreVideoVolume(video);
        return;
      }

      if (!dubAudioEl) {
        dubAudioEl = new Audio();
        try {
          const AudioCtx = window.AudioContext || window.webkitAudioContext;
          if (AudioCtx) {
            const actx = new AudioCtx();
            const src = actx.createMediaElementSource(dubAudioEl);
            const gainNode = actx.createGain();
            const comp = actx.createDynamicsCompressor();
            comp.threshold.value = -3;
            comp.ratio.value = 8;
            src.connect(gainNode);
            gainNode.connect(comp);
            comp.connect(actx.destination);
            dubAudioEl._ytcsCtx = actx;
            dubAudioEl._ytcsGain = gainNode;
          }
        } catch {}
      }
      const voiceGain = Math.max(0.5, Math.min(2.0, Number(subStyle.dubVoiceGain ?? 1.3)));
      if (dubAudioEl._ytcsGain && dubAudioEl._ytcsCtx) {
        if (dubAudioEl._ytcsCtx.state === 'suspended') {
          dubAudioEl._ytcsCtx.resume().catch(() => {});
        }
        dubAudioEl._ytcsGain.gain.value = voiceGain;
        dubAudioEl.volume = 1.0;
      } else {
        dubAudioEl.volume = Math.min(1.0, voiceGain);
      }
      dubAudioEl.onended = () => {
        if (mySeq === speakSeq) restoreVideoVolume(video);
      };
      dubAudioEl.onerror = () => {
        if (mySeq === speakSeq) restoreVideoVolume(video);
      };
      dubAudioEl.src = audioUrl;

      const ytRate = Math.max(0.5, Number(video.playbackRate) || 1);
      const userSpeed = Math.max(0.7, Math.min(2.0, Number(subStyle.dubSpeed) || 1.0));
      const sourceRateFactor = voice.startsWith('vieneu:') ? 1.0 : 1.15;
      const baseAudioRate = (userSpeed * ytRate) / sourceRateFactor;
      let exactAudioRate = baseAudioRate;
      const audioDur = duration > 0 ? duration : (Number.isFinite(dubAudioEl.duration) ? dubAudioEl.duration : 0);
      if (audioDur > 0) {
        const nextCue = subBundle?.cues?.[idx + 1];
        const maxEndMs = nextCue
          ? Math.min((cue.endMs || 0) + 1200, Math.max(cue.endMs || 0, (nextCue.startMs || 0) - 80))
          : (cue.endMs || 0) + 1200;
        const remCueMs = Math.max(300, maxEndMs - video.currentTime * 1000);
        const availSec = remCueMs / 1000 / ytRate;
        const fitAudioRate = audioDur / Math.max(0.25, availSec * 0.96);
        exactAudioRate = Math.max(baseAudioRate, fitAudioRate);
      } else {
        exactAudioRate = computeDubRate(text, cue, video) / sourceRateFactor;
      }
      dubAudioEl.playbackRate = Math.min(2.5, Math.max(0.7, exactAudioRate));
      duckVideoVolume(video);
      await dubAudioEl.play();
    } catch {
      if (mySeq !== speakSeq || !dubEnabled || video.paused) {
        restoreVideoVolume(video);
        return;
      }
      speakCueBrowser(text, cue, video, mySeq);
    }
  }

  function tickSubtitles() {
    const video = document.querySelector('video.html5-main-video') || document.querySelector('video');
    if (video && subBundle?.cues?.length && (subMode !== 'off' || dubEnabled)) {
      const nowMs = video.currentTime * 1000;
      const idx = subBundle.cues.findIndex((c) => nowMs >= c.startMs && nowMs <= c.endMs);
      const cue = idx >= 0 ? subBundle.cues[idx] : null;

      if (subMode !== 'off') {
        const overlay = ensureSubOverlay();
        if (overlay) {
          if (!cue) {
            overlay.innerHTML = '';
          } else {
            const a = (Number(subStyle.bgOpacity ?? 80) / 100);
            const trBg = hexToRgba(subStyle.bgColor, a.toFixed(2));
            const origBg = hexToRgba(subStyle.bgColor, (a * 0.85).toFixed(2));
            const trSize = Number(subStyle.fontSize || 18);
            const origSize = Math.max(11, Math.round(trSize * 0.78));
            const origHtml =
              subMode === 'bilingual'
                ? `<div style="background:${origBg};color:#e2e8f0;padding:3px 10px;border-radius:6px;font-size:${origSize}px;line-height:1.35;">${cue.original.replace(/</g, '&lt;')}</div>`
                : '';
            const trHtml = `<div style="background:${trBg};color:${subStyle.textColor || '#ffffff'};padding:5px 12px;border-radius:8px;font-size:${trSize}px;font-weight:600;line-height:1.35;text-shadow:0 1px 3px #000;">${cue.translated.replace(/</g, '&lt;')}</div>`;
            overlay.innerHTML = origHtml + trHtml;
          }
        }
      }

      if (dubEnabled) {
        bindVideoDubEvents(video);
        if (video.paused) {
          if (dubAudioEl && !dubAudioEl.paused) {
            pauseDubbingSpeech(video);
          }
        } else if (idx >= 0 && idx !== lastSpokenCueIdx) {
          lastSpokenCueIdx = idx;
          speakCue(cue, video, idx);
        }
      }
    }
    if (subMode !== 'off' || dubEnabled) {
      syncRaf = requestAnimationFrame(tickSubtitles);
    } else {
      syncRaf = null;
    }
  }

  async function waitForTranslateJob(jobId, videoId) {
    while (true) {
      await new Promise((r) => setTimeout(r, 1500));
      const { job } = await sendBg('GET_JOB', { id: jobId });
      if (!job) throw new Error('Translation job disappeared');

      // Progressive subtitle loading: start showing translated cues as soon as chunk 1 / batch 1 is cached
      if (job.progress >= 20 || job.status === 'done') {
        try {
          const partial = await sendBg('GET_SUBTITLES', { videoId });
          if (partial?.cues?.length) {
            subBundle = partial;
            if (subMode === 'off') {
              subMode = 'bilingual';
              if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
            }
          }
        } catch {
          // Not cached yet
        }
      }

      if (job.status === 'done') {
        return subBundle || (await sendBg('GET_SUBTITLES', { videoId }));
      }
      if (job.status === 'error' || job.status === 'cancelled') {
        throw new Error(job.error || 'Translation failed');
      }
      subBtnEl.title = job.stageText || 'Translating...';
      const txtEl = subBtnEl.querySelector('.txt');
      if (txtEl) txtEl.textContent = `AI CC: ${job.progress || 0}%`;
    }
  }

  async function ensureTranslatedBundle() {
    const videoId = getVideoId();
    if (!videoId) return null;
    if (subBundle && subBundle.videoId === videoId) return subBundle;
    const txtEl = subBtnEl.querySelector('.txt');
    if (txtEl) txtEl.textContent = 'Translating...';
    subBtnEl.dataset.state = 'active';
    const res = await sendBg('TRANSLATE_VIDEO', {
      payload: {
        url: getCleanVideoUrl(videoId),
        videoId,
        title: getVideoTitle(),
      },
    });
    if (res?.cached && res?.bundle) {
      subBundle = res.bundle;
      return subBundle;
    }
    if (res?.job) {
      subBundle = await waitForTranslateJob(res.job.id, videoId);
      return subBundle;
    }
    subBundle = res;
    return subBundle;
  }

  let subCycleStartMode = null;

  function syncSubModeToStorage(mode) {
    if (mode !== 'bilingual' && mode !== 'translated') return;
    if (subStyle.defaultMode === mode) return;
    subStyle = { ...subStyle, defaultMode: mode };
    chrome.storage.local.set({ subStyle });
  }

  async function onSubClick() {
    if (subMode === 'off') {
      try {
        await ensureTranslatedBundle();
        subCycleStartMode = subStyle.defaultMode || 'bilingual';
        subMode = subCycleStartMode;
        updateSubButtons();
        if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
      } catch (err) {
        subBtnEl.title = err.message;
        subBtnEl.dataset.state = 'error';
        const txtEl = subBtnEl.querySelector('.txt');
        if (txtEl) txtEl.textContent = 'AI CC: Error';
      }
      return;
    }
    if (subCycleStartMode && subMode === subCycleStartMode) {
      subMode = subCycleStartMode === 'bilingual' ? 'translated' : 'bilingual';
      syncSubModeToStorage(subMode);
      updateSubButtons();
      return;
    }
    subMode = 'off';
    subCycleStartMode = null;
    const overlay = document.getElementById(SUB_OVERLAY_ID);
    if (overlay) overlay.innerHTML = '';
    updateSubButtons();
  }

  async function onDubClick() {
    if (!dubEnabled) {
      try {
        await ensureTranslatedBundle();
        const video = document.querySelector('video.html5-main-video') || document.querySelector('video');
        const wasPlaying = Boolean(video && !video.paused);
        const nowMs = (video?.currentTime || 0) * 1000;
        const curIdx = Math.max(0, subBundle?.cues?.findIndex((c) => c.endMs >= nowMs) ?? 0);
        preparedFromIdx = -1;
        preparedUpToIdx = -1;
        lastPrefetchedIdx = -1;
        const voice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
        if (voice !== 'browser' && dubBtnEl) {
          if (wasPlaying) video.pause();
          dubBtnEl.dataset.state = 'working';
          const txt = dubBtnEl.querySelector('.txt');
          if (txt) txt.textContent = 'Dub: Buffering...';
          await prepareBatchAhead(curIdx, 8);
          // Pre-warm first 4 cues directly into browser RAM cache for instant 0ms playback
          const preloadTasks = [];
          for (let i = curIdx; i < Math.min(subBundle?.cues?.length || 0, curIdx + 4); i += 1) {
            const c = subBundle.cues[i];
            const t = c?.translated?.trim();
            if (hasSpeakableText(t)) {
              preloadTasks.push(fetchEdgeTtsAudio(t, resolveCueVoice(c), '+15%').catch(() => {}));
            }
          }
          if (preloadTasks.length) await Promise.all(preloadTasks);
          if (wasPlaying && video.paused) {
            video.play().catch(() => {});
          }
        }
        dubEnabled = true;
        if (subMode === 'off') subMode = 'translated';
        lastSpokenCueIdx = -1;
        updateSubButtons();
        if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
      } catch (err) {
        dubBtnEl.title = err.message;
        dubBtnEl.dataset.state = 'error';
      }
      return;
    }
    dubEnabled = false;
    const video = document.querySelector('video.html5-main-video') || document.querySelector('video');
    stopDubbingSpeech(video);
    updateSubButtons();
  }

  function hasYouTubePlayerError() {
    const errorRenderer = document.querySelector(
      '#movie_player .ytp-error, ytd-watch-flexy ytd-player-error-message-renderer, #player ytd-player-error-message-renderer',
    );
    if (!errorRenderer) return false;
    const style = window.getComputedStyle(errorRenderer);
    return style.display !== 'none' && style.visibility !== 'hidden';
  }

  function maybeMountCloudPlayer(force = false) {
    const videoId = getVideoId();
    if (!videoId || (!isSavedInCloud && !force)) return;
    if (document.getElementById(PLAYER_FRAME_ID)) return;
    if (!force && !hasYouTubePlayerError()) return;

    const ytVideo = document.querySelector('video.html5-main-video') || document.querySelector('video');
    if (ytVideo) {
      try {
        ytVideo.pause();
      } catch {}
      if (!ytVideo._ytcsCloudGuard) {
        ytVideo._ytcsCloudGuard = true;
        ytVideo.addEventListener('play', () => {
          if (document.getElementById(PLAYER_FRAME_ID)) {
            try {
              ytVideo.pause();
            } catch {}
          }
        });
      }
      stopDubbingSpeech(ytVideo, true);
      dubEnabled = false;
      updateSubButtons();
    }

    const container =
      document.querySelector('#movie_player') ||
      document.querySelector('#player-container-inner') ||
      document.querySelector('#player');
    if (!container) return;

    if (window.getComputedStyle(container).position === 'static') {
      container.style.position = 'relative';
    }

    const frame = document.createElement('iframe');
    frame.id = PLAYER_FRAME_ID;
    frame.src = chrome.runtime.getURL(`player.html?videoId=${encodeURIComponent(videoId)}`);
    frame.allow = 'autoplay; fullscreen';
    Object.assign(frame.style, {
      position: 'absolute',
      inset: '0',
      width: '100%',
      height: '100%',
      border: 'none',
      zIndex: '9999',
      background: '#000',
    });

    container.appendChild(frame);
    setTimeout(() => {
      try {
        frame.focus();
      } catch {}
    }, 50);
  }

  async function syncCurrentVideoJob() {
    const videoId = getVideoId();
    stopPolling();
    removeCloudPlayer();
    removeSubOverlay();
    subBundle = null;
    subMode = 'off';
    dubEnabled = false;
    updateSubButtons();
    currentJob = null;
    isSavedInCloud = false;
    if (!videoId) {
      renderJobState(null);
      return;
    }
    try {
      const [{ jobs }, saved] = await Promise.all([
        sendBg('LIST_JOBS'),
        sendBg('GET_SAVED_IDS').catch(() => ({ ids: [] })),
      ]);
      const match = (jobs || []).find((j) => j.videoId === videoId && (j.kind || 'cloud') === 'cloud');
      if (match && (ACTIVE_STATUSES.has(match.status) || match.status === 'done')) {
        currentJob = match;
        isSavedInCloud = match.status === 'done' || (saved?.ids || []).includes(videoId);
        renderJobState(match);
        if (ACTIVE_STATUSES.has(match.status)) {
          startPolling(match.id);
        }
        maybeMountCloudPlayer();
        return;
      }
      if ((saved?.ids || []).includes(videoId)) {
        isSavedInCloud = true;
        const syncedDone = { status: 'done', remote: 'Google Drive (synced)' };
        currentJob = syncedDone;
        renderJobState(syncedDone);
        maybeMountCloudPlayer();
        return;
      }
    } catch {
      // Server may be offline; keep idle button
    }
    renderJobState(null);
  }

  function ensureButton() {
    if (!getVideoId()) return;
    maybeMountCloudPlayer();
    ensurePlayerBarControls();
    const container = findActionContainer();
    if (!container) return;

    const existing = document.getElementById(HOST_ID);
    if (existing && existing.parentElement === container) return;
    if (existing) existing.remove();

    const host = createButtonHost();
    container.appendChild(host);
    renderJobState(currentJob);
  }

  const observer = new MutationObserver(() => {
    ensureButton();
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });

  window.addEventListener('message', (e) => {
    if (e.data?.type === 'YTCS_CLOSE_CLOUD_PLAYER') {
      const frame = document.getElementById(PLAYER_FRAME_ID);
      if (frame) frame.remove();
    }
  });

  window.addEventListener('keydown', (e) => {
    if (!getVideoId()) return;
    if (document.getElementById(PLAYER_FRAME_ID)) return;
    const t = e.target;
    if (t && (['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName) || t.isContentEditable)) return;
    if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
      if (e.code === 'KeyC') {
        e.preventDefault();
        e.stopPropagation();
        onSubClick();
      } else if (e.code === 'KeyD') {
        e.preventDefault();
        e.stopPropagation();
        onDubClick();
      }
    }
  }, true);

  window.addEventListener('yt-navigate-finish', () => {
    ensureButton();
    syncCurrentVideoJob();
  });

  ensureButton();
  syncCurrentVideoJob();
})();
