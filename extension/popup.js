const statusBadge = document.getElementById('statusBadge');
const openAppBtn = document.getElementById('openAppBtn');
const toolsRow = document.getElementById('toolsRow');
const remoteInput = document.getElementById('remoteInput');
const remoteSuggestions = document.getElementById('remoteSuggestions');
const browseBtn = document.getElementById('browseBtn');
const browserBox = document.getElementById('browserBox');
const browserPathEl = document.getElementById('browserPath');
const upDirBtn = document.getElementById('upDirBtn');
const useDirBtn = document.getElementById('useDirBtn');
const dirList = document.getElementById('dirList');
const newDirInput = document.getElementById('newDirInput');
const createDirBtn = document.getElementById('createDirBtn');
const groupByChannelInput = document.getElementById('groupByChannelInput');
const pathPreview = document.getElementById('pathPreview');
const qualitySelect = document.getElementById('qualitySelect');
const serverUrlInput = document.getElementById('serverUrlInput');
const saveSettingsBtn = document.getElementById('saveSettingsBtn');
const settingsMsg = document.getElementById('settingsMsg');
const jobsList = document.getElementById('jobsList');
const refreshBtn = document.getElementById('refreshBtn');

const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);
let detectedRemotes = [];
let currentBrowsePath = '';

function sendBg(type, payload = {}) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage({ type, ...payload }, (res) => {
      if (chrome.runtime.lastError) return reject(new Error(chrome.runtime.lastError.message));
      if (!res?.ok) return reject(new Error(res?.error || 'Request failed'));
      resolve(res.data);
    });
  });
}

function escapeHtml(str) {
  return String(str)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function joinRemote(base, sub) {
  const clean = String(base || '').replace(/\/+$/, '');
  if (!sub) return clean;
  return clean.endsWith(':') ? `${clean}${sub}` : `${clean}/${sub}`;
}

function parentRemote(remotePath) {
  const colonIdx = remotePath.indexOf(':');
  if (colonIdx === -1) return remotePath;
  const prefix = remotePath.slice(0, colonIdx + 1);
  const rest = remotePath.slice(colonIdx + 1).replace(/\/+$/, '');
  if (!rest) return prefix;
  const parts = rest.split('/');
  parts.pop();
  return parts.length ? `${prefix}${parts.join('/')}` : prefix;
}

function updatePathPreview() {
  const base = (remoteInput.value || 'gdrive:YouTube Saved').trim().replace(/\/+$/, '');
  if (groupByChannelInput.checked) {
    pathPreview.textContent = `Saves to: ${joinRemote(base, '<Channel Name>')}/`;
  } else {
    pathPreview.textContent = `Saves to: ${base}/`;
  }
}

function renderTools(tools = {}) {
  const items = [
    ['yt-dlp', tools.ytDlp],
    ['ffmpeg', tools.ffmpeg],
    ['rclone', tools.rclone],
  ];
  toolsRow.innerHTML = items
    .map(
      ([name, ok]) =>
        `<span class="${ok ? 'tool-ok' : 'tool-missing'}">${ok ? '✓' : '✗'} ${name}</span>`,
    )
    .join('');
}

function renderJobs(jobs = []) {
  if (!jobs.length) {
    jobsList.innerHTML = '<p class="empty">No jobs yet.</p>';
    return;
  }

  jobsList.innerHTML = jobs
    .map((job) => {
      const isActive = ACTIVE_STATUSES.has(job.status);
      const pct = job.status === 'done' ? 100 : job.progress || 0;
      const actionBtn = isActive
        ? `<button class="btn-ghost" data-cancel="${escapeHtml(job.id)}">Cancel</button>`
        : `<button class="btn-ghost" data-delete="${escapeHtml(job.id)}">Remove</button>`;

      return `
        <div class="job-item">
          <div class="job-top">
            <div class="job-title">${escapeHtml(job.title || job.url)}</div>
            ${actionBtn}
          </div>
          <div class="job-meta">${escapeHtml(job.stageText || job.status)} · ${escapeHtml(job.remote || '')}</div>
          ${job.error ? `<div class="job-error">${escapeHtml(job.error)}</div>` : ''}
          <div class="bar"><div class="bar-fill ${job.status === 'done' ? 'done' : ''}" style="width:${pct}%"></div></div>
        </div>
      `;
    })
    .join('');
}

async function openDirectory(remotePath) {
  currentBrowsePath = remotePath;
  browserPathEl.textContent = remotePath;
  dirList.innerHTML = '<p class="empty">Loading folders...</p>';
  try {
    const { dirs } = await sendBg('LIST_DIRS', { path: remotePath });
    if (!dirs.length) {
      dirList.innerHTML = '<p class="empty">No subfolders here.</p>';
      return;
    }
    dirList.innerHTML = dirs
      .map(
        (name) =>
          `<button type="button" class="dir-item" data-dir="${escapeHtml(name)}">📁 ${escapeHtml(name)}</button>`,
      )
      .join('');
  } catch (err) {
    dirList.innerHTML = `<p class="job-error">${escapeHtml(err.message)}</p>`;
  }
}

async function loadState() {
  const { serverUrl } = await sendBg('GET_SERVER_URL');
  serverUrlInput.value = serverUrl;

  try {
    const health = await sendBg('HEALTH');
    statusBadge.textContent = 'Server Online';
    statusBadge.className = 'badge badge-online';
    renderTools(health.tools);
    detectedRemotes = health.remotes || [];

    if (health.config) {
      if (!remoteInput.matches(':focus')) remoteInput.value = health.config.rcloneRemote || '';
      groupByChannelInput.checked = Boolean(health.config.groupByChannel);
      qualitySelect.value = health.config.quality || 'best';
      updatePathPreview();
    }

    remoteSuggestions.innerHTML = detectedRemotes
      .map((r) => `<option value="${escapeHtml(r)}YouTube Saved"></option>`)
      .join('');

    const { jobs } = await sendBg('LIST_JOBS');
    renderJobs(jobs);
  } catch {
    statusBadge.textContent = 'Server Offline';
    statusBadge.className = 'badge badge-offline';
    toolsRow.innerHTML = '<span>Start local server: <code>node server/index.js</code></span>';
    jobsList.innerHTML = '<p class="empty">Cannot connect to local server.</p>';
  }
}

browseBtn.addEventListener('click', async () => {
  const willShow = browserBox.classList.contains('hidden');
  browserBox.classList.toggle('hidden', !willShow);
  if (willShow) {
    const startPath = remoteInput.value.trim() || detectedRemotes[0] || 'gdrive:';
    await openDirectory(startPath);
  }
});

upDirBtn.addEventListener('click', () => {
  openDirectory(parentRemote(currentBrowsePath));
});

useDirBtn.addEventListener('click', () => {
  remoteInput.value = currentBrowsePath;
  updatePathPreview();
  browserBox.classList.add('hidden');
});

dirList.addEventListener('click', (e) => {
  const dirName = e.target.getAttribute('data-dir');
  if (dirName) {
    openDirectory(joinRemote(currentBrowsePath, dirName));
  }
});

createDirBtn.addEventListener('click', async () => {
  const name = newDirInput.value.trim();
  if (!name) return;
  const target = joinRemote(currentBrowsePath, name);
  try {
    await sendBg('CREATE_DIR', { path: target });
    newDirInput.value = '';
    await openDirectory(currentBrowsePath);
  } catch (err) {
    dirList.innerHTML = `<p class="job-error">${escapeHtml(err.message)}</p>`;
  }
});

remoteInput.addEventListener('input', updatePathPreview);
groupByChannelInput.addEventListener('change', updatePathPreview);

saveSettingsBtn.addEventListener('click', async () => {
  settingsMsg.textContent = '';
  try {
    await sendBg('SET_SERVER_URL', { serverUrl: serverUrlInput.value });
    await sendBg('SAVE_CONFIG', {
      payload: {
        rcloneRemote: remoteInput.value.trim(),
        groupByChannel: groupByChannelInput.checked,
        quality: qualitySelect.value,
      },
    });
    settingsMsg.textContent = 'Saved!';
    await loadState();
  } catch (err) {
    settingsMsg.textContent = err.message;
  }
});

jobsList.addEventListener('click', async (e) => {
  const cancelId = e.target.getAttribute('data-cancel');
  const deleteId = e.target.getAttribute('data-delete');
  if (cancelId) {
    await sendBg('CANCEL_JOB', { id: cancelId });
    await loadState();
  } else if (deleteId) {
    await sendBg('DELETE_JOB', { id: deleteId });
    await loadState();
  }
});

refreshBtn.addEventListener('click', loadState);
openAppBtn.addEventListener('click', async () => {
  try {
    await sendBg('SHOW_APP');
  } catch {
    // Ignore if standalone Node server (without Desktop UI) is used
  }
});

loadState();
setInterval(loadState, 2000);
