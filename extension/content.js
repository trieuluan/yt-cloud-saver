(() => {
  const HOST_ID = 'yt-cloud-saver-host';
  const PLAYER_FRAME_ID = 'yt-cloud-saver-player';
  const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);

  let currentJob = null;
  let isSavedInCloud = false;
  let pollTimer = null;
  let buttonEl = null;
  let labelEl = null;
  let progressEl = null;

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

  function createButtonHost() {
    const host = document.createElement('div');
    host.id = HOST_ID;
    host.style.display = 'inline-flex';
    host.style.alignItems = 'center';
    host.style.marginLeft = '8px';

    const shadow = host.attachShadow({ mode: 'closed' });
    shadow.innerHTML = `
      <style>
        :host {
          font-family: "Roboto", "Arial", sans-serif;
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
      </style>
      <button type="button" data-state="idle" title="Save video to Cloud via local server">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M19.35 10.04A7.49 7.49 0 0 0 12 4C9.11 4 6.6 5.64 5.35 8.04A5.994 5.994 0 0 0 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM17 13l-5 5-5-5h3V9h4v4h3z"/>
        </svg>
        <span class="label">Save to Cloud</span>
        <span class="progress"></span>
      </button>
    `;

    buttonEl = shadow.querySelector('button');
    labelEl = shadow.querySelector('.label');
    progressEl = shadow.querySelector('.progress');

    buttonEl.addEventListener('click', onButtonClick);
    return host;
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
      buttonEl.title = `Saved to ${job.remote} (Click to save again)`;
      labelEl.textContent = 'Saved ✓';
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

  function hasYouTubePlayerError() {
    const errorRenderer = document.querySelector(
      '#movie_player .ytp-error, ytd-watch-flexy ytd-player-error-message-renderer, #player ytd-player-error-message-renderer',
    );
    if (!errorRenderer) return false;
    const style = window.getComputedStyle(errorRenderer);
    return style.display !== 'none' && style.visibility !== 'hidden';
  }

  function maybeMountCloudPlayer() {
    const videoId = getVideoId();
    if (!videoId || !isSavedInCloud) return;
    if (document.getElementById(PLAYER_FRAME_ID)) return;
    if (!hasYouTubePlayerError()) return;

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
    frame.allowFullscreen = true;
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
  }

  async function syncCurrentVideoJob() {
    const videoId = getVideoId();
    stopPolling();
    removeCloudPlayer();
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
      const match = (jobs || []).find((j) => j.videoId === videoId);
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

  window.addEventListener('yt-navigate-finish', () => {
    ensureButton();
    syncCurrentVideoJob();
  });

  ensureButton();
  syncCurrentVideoJob();
})();
