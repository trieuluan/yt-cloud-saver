const statusBadge = document.getElementById('statusBadge');
const openAppBtn = document.getElementById('openAppBtn');
const toolsRow = document.getElementById('toolsRow');

const tabSub = document.getElementById('tabSub');
const tabCloud = document.getElementById('tabCloud');
const viewSub = document.getElementById('viewSub');
const viewCloud = document.getElementById('viewCloud');

const modeBtns = document.querySelectorAll('.mode-btn');
const szBtns = document.querySelectorAll('.sz-btn');
const sizeSlider = document.getElementById('sizeSlider');
const sizeLabel = document.getElementById('sizeLabel');
const clrBtns = document.querySelectorAll('.clr-btn');
const textColorPicker = document.getElementById('textColorPicker');
const bgBtns = document.querySelectorAll('.bg-btn');
const bgColorPicker = document.getElementById('bgColorPicker');
const opacitySlider = document.getElementById('opacitySlider');
const opacityLabel = document.getElementById('opacityLabel');
const prevOrig = document.getElementById('prevOrig');
const prevTr = document.getElementById('prevTr');
const dubVolSelect = document.getElementById('dubVolSelect');

const remoteInput = document.getElementById('remoteInput');
const dubVoiceSelect = document.getElementById('dubVoiceSelect');
const dubSpeedSelect = document.getElementById('dubSpeedSelect');
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
const settingsMsg = document.getElementById('settingsMsg');
const jobsList = document.getElementById('jobsList');
const refreshBtn = document.getElementById('refreshBtn');

const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);
const DEFAULT_SUB_STYLE = {
  defaultMode: 'bilingual',
  fontSize: 18,
  textColor: '#ffffff',
  bgColor: '#000000',
  bgOpacity: 80,
  dubVoice: 'vi-VN-HoaiMyNeural',
  dubSpeed: 1.0,
  dubVolume: 0.15,
};

let subStyle = { ...DEFAULT_SUB_STYLE };
let detectedRemotes = [];
let currentBrowsePath = '';
let configHydrated = false;

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

function hexToRgba(hex, alpha) {
  const clean = (hex || '#000000').replace('#', '');
  const r = parseInt(clean.slice(0, 2), 16) || 0;
  const g = parseInt(clean.slice(2, 4), 16) || 0;
  const b = parseInt(clean.slice(4, 6), 16) || 0;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function renderSubStyle() {
  modeBtns.forEach((b) => b.classList.toggle('active', b.dataset.mode === subStyle.defaultMode));
  prevOrig.classList.toggle('hidden', subStyle.defaultMode !== 'bilingual');

  sizeSlider.value = subStyle.fontSize;
  sizeLabel.textContent = `${subStyle.fontSize}px`;
  szBtns.forEach((b) => b.classList.toggle('active', Number(b.dataset.size) === Number(subStyle.fontSize)));

  textColorPicker.value = subStyle.textColor;
  clrBtns.forEach((b) => b.classList.toggle('active', b.dataset.color.toLowerCase() === subStyle.textColor.toLowerCase()));

  bgColorPicker.value = subStyle.bgColor;
  bgBtns.forEach((b) => b.classList.toggle('active', b.dataset.bg.toLowerCase() === subStyle.bgColor.toLowerCase()));

  opacitySlider.value = subStyle.bgOpacity;
  opacityLabel.textContent = `${subStyle.bgOpacity}%`;
  dubVoiceSelect.value = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
  dubSpeedSelect.value = String(subStyle.dubSpeed ?? 1.0);
  dubVolSelect.value = String(subStyle.dubVolume);

  const a = subStyle.bgOpacity / 100;
  prevTr.style.fontSize = `${subStyle.fontSize}px`;
  prevTr.style.color = subStyle.textColor;
  prevTr.style.background = hexToRgba(subStyle.bgColor, a);
  prevOrig.style.fontSize = `${Math.max(11, Math.round(subStyle.fontSize * 0.78))}px`;
  prevOrig.style.background = hexToRgba(subStyle.bgColor, (a * 0.85).toFixed(2));
}

async function saveSubStyle() {
  renderSubStyle();
  await chrome.storage.local.set({ subStyle });
}

async function loadSubStyle() {
  const res = await chrome.storage.local.get({ subStyle: DEFAULT_SUB_STYLE });
  subStyle = { ...DEFAULT_SUB_STYLE, ...(res.subStyle || {}) };
  renderSubStyle();
}

chrome.storage.onChanged.addListener((changes, area) => {
  if (area === 'local' && changes.subStyle?.newValue) {
    subStyle = { ...DEFAULT_SUB_STYLE, ...changes.subStyle.newValue };
    renderSubStyle();
  }
});

function switchTab(isSub) {
  viewSub.classList.toggle('hidden', !isSub);
  viewCloud.classList.toggle('hidden', isSub);
  tabSub.classList.toggle('active', isSub);
  tabCloud.classList.toggle('active', !isSub);
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
  pathPreview.textContent = groupByChannelInput.checked
    ? `Saves to: ${joinRemote(base, '<Channel Name>')}/`
    : `Saves to: ${base}/`;
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
  const activeCount = jobs.filter((j) => ACTIVE_STATUSES.has(j.status)).length;
  tabCloud.textContent = activeCount > 0 ? `Cloud & Jobs (${activeCount})` : 'Cloud & Jobs';

  if (!jobs.length) {
    jobsList.innerHTML = '<p class="empty">No jobs yet.</p>';
    return;
  }

  jobsList.innerHTML = jobs
    .map((job) => {
      const kind = job.kind || 'cloud';
      const isActive = ACTIVE_STATUSES.has(job.status);
      const pct = job.status === 'done' ? 100 : job.progress || 0;
      const badge =
        kind === 'translate'
          ? '<span class="kind-pill kind-translate">Translate</span>'
          : '<span class="kind-pill kind-cloud">Cloud</span>';
      const barCls = job.status === 'done' ? 'done' : kind === 'translate' ? 'translate' : '';
      const actionBtn = isActive
        ? `<button class="btn-ghost" data-cancel="${escapeHtml(job.id)}">Cancel</button>`
        : `<button class="btn-ghost" data-delete="${escapeHtml(job.id)}">Remove</button>`;

      return `
        <div class="job-item">
          <div class="job-top">
            <div class="job-title">${badge}${escapeHtml(job.title || job.url)}</div>
            ${actionBtn}
          </div>
          <div class="job-meta">${escapeHtml(job.stageText || job.status)} · ${escapeHtml(job.remote || '')}</div>
          ${job.error ? `<div class="job-error">${escapeHtml(job.error)}</div>` : ''}
          <div class="bar"><div class="bar-fill ${barCls}" style="width:${pct}%"></div></div>
        </div>
      `;
    })
    .join('');
}

async function openDirectory(remotePath, allowFallback = true) {
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
    const msg = String(err);
    const rootPrefix = remotePath.includes(':') ? `${remotePath.split(':')[0]}:` : '';
    if (allowFallback && rootPrefix && remotePath !== rootPrefix && msg.includes('directory not found')) {
      return openDirectory(rootPrefix, false);
    }
    dirList.innerHTML = `<p class="job-error">${escapeHtml(msg)}</p>`;
  }
}

async function persistCloudSettings() {
  try {
    await sendBg('SET_SERVER_URL', { serverUrl: serverUrlInput.value });
    await sendBg('SAVE_CONFIG', {
      payload: {
        rcloneRemote: remoteInput.value.trim(),
        groupByChannel: groupByChannelInput.checked,
        quality: qualitySelect.value,
      },
    });
    settingsMsg.textContent = '✓ Auto-saved';
    setTimeout(() => {
      settingsMsg.textContent = '';
    }, 1500);
  } catch (err) {
    settingsMsg.textContent = err.message;
  }
}

async function loadState() {
  if (!configHydrated) {
    const { serverUrl } = await sendBg('GET_SERVER_URL');
    serverUrlInput.value = serverUrl;
  }

  try {
    const health = await sendBg('HEALTH');
    statusBadge.textContent = 'Online';
    statusBadge.className = 'badge badge-online';
    renderTools(health.tools);
    detectedRemotes = health.remotes || [];

    if (health.config && !configHydrated) {
      configHydrated = true;
      remoteInput.value = health.config.rcloneRemote || '';
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
    statusBadge.textContent = 'Offline';
    statusBadge.className = 'badge badge-offline';
    toolsRow.innerHTML = '<span>Open YT Cloud Saver Desktop App</span>';
    jobsList.innerHTML = '<p class="empty">Cannot connect to local server.</p>';
  }
}

tabSub.addEventListener('click', () => switchTab(true));
tabCloud.addEventListener('click', () => switchTab(false));

modeBtns.forEach((b) =>
  b.addEventListener('click', () => {
    subStyle.defaultMode = b.dataset.mode;
    saveSubStyle();
  }),
);
szBtns.forEach((b) =>
  b.addEventListener('click', () => {
    subStyle.fontSize = Number(b.dataset.size);
    saveSubStyle();
  }),
);
sizeSlider.addEventListener('input', () => {
  subStyle.fontSize = Number(sizeSlider.value);
  saveSubStyle();
});
clrBtns.forEach((b) =>
  b.addEventListener('click', () => {
    subStyle.textColor = b.dataset.color;
    saveSubStyle();
  }),
);
textColorPicker.addEventListener('input', () => {
  subStyle.textColor = textColorPicker.value;
  saveSubStyle();
});
bgBtns.forEach((b) =>
  b.addEventListener('click', () => {
    subStyle.bgColor = b.dataset.bg;
    saveSubStyle();
  }),
);
bgColorPicker.addEventListener('input', () => {
  subStyle.bgColor = bgColorPicker.value;
  saveSubStyle();
});
opacitySlider.addEventListener('input', () => {
  subStyle.bgOpacity = Number(opacitySlider.value);
  saveSubStyle();
});
dubVoiceSelect.addEventListener('change', () => {
  subStyle.dubVoice = dubVoiceSelect.value;
  saveSubStyle();
});
dubSpeedSelect.addEventListener('change', () => {
  subStyle.dubSpeed = Number(dubSpeedSelect.value);
  saveSubStyle();
});
dubVolSelect.addEventListener('change', () => {
  subStyle.dubVolume = Number(dubVolSelect.value);
  saveSubStyle();
});

browseBtn.addEventListener('click', async () => {
  const willShow = browserBox.classList.contains('hidden');
  browserBox.classList.toggle('hidden', !willShow);
  if (willShow) {
    const startPath = remoteInput.value.trim() || detectedRemotes[0] || 'gdrive:';
    await openDirectory(startPath);
  }
});

upDirBtn.addEventListener('click', () => openDirectory(parentRemote(currentBrowsePath)));

useDirBtn.addEventListener('click', () => {
  remoteInput.value = currentBrowsePath;
  updatePathPreview();
  browserBox.classList.add('hidden');
  persistCloudSettings();
});

dirList.addEventListener('click', (e) => {
  const dirName = e.target.getAttribute('data-dir');
  if (dirName) openDirectory(joinRemote(currentBrowsePath, dirName));
});

createDirBtn.addEventListener('click', async () => {
  const name = newDirInput.value.trim();
  if (!name) return;
  try {
    await sendBg('CREATE_DIR', { path: joinRemote(currentBrowsePath, name) });
    newDirInput.value = '';
    await openDirectory(currentBrowsePath);
  } catch (err) {
    dirList.innerHTML = `<p class="job-error">${escapeHtml(err.message)}</p>`;
  }
});

remoteInput.addEventListener('input', updatePathPreview);
remoteInput.addEventListener('change', persistCloudSettings);
groupByChannelInput.addEventListener('change', () => {
  updatePathPreview();
  persistCloudSettings();
});
qualitySelect.addEventListener('change', persistCloudSettings);
serverUrlInput.addEventListener('change', persistCloudSettings);

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
    // Ignore if standalone server
  }
});

loadSubStyle();
loadState();
setInterval(loadState, 2000);
