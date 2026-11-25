const { invoke } = window.__TAURI__.core;

const sysBtn = document.getElementById('sysBtn');
const sysDrawer = document.getElementById('sysDrawer');
const toolsRow = document.getElementById('toolsRow');
const installToolsBtn = document.getElementById('installToolsBtn');
const tabActivity = document.getElementById('tabActivity');
const tabSettings = document.getElementById('tabSettings');
const viewActivity = document.getElementById('viewActivity');
const viewSettings = document.getElementById('viewSettings');

const setupBanner = document.getElementById('setupBanner');
const bannerConnectBtn = document.getElementById('bannerConnectBtn');
const quickSaveForm = document.getElementById('quickSaveForm');
const quickUrlInput = document.getElementById('quickUrlInput');
const quickSaveBtn = document.getElementById('quickSaveBtn');
const quickSaveError = document.getElementById('quickSaveError');
const syncStatusText = document.getElementById('syncStatusText');
const syncDriveBtn = document.getElementById('syncDriveBtn');
const jobsList = document.getElementById('jobsList');

const accountAvatar = document.getElementById('accountAvatar');
const remotesText = document.getElementById('remotesText');
const connectDriveBtn = document.getElementById('connectDriveBtn');
const connectHint = document.getElementById('connectHint');

const remoteInput = document.getElementById('remoteInput');
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
const cookiesSelect = document.getElementById('cookiesSelect');
const settingsMsg = document.getElementById('settingsMsg');

const guideBtn = document.getElementById('guideBtn');
const guideModal = document.getElementById('guideModal');
const closeGuideBtn = document.getElementById('closeGuideBtn');
const openExtFolderBtn = document.getElementById('openExtFolderBtn');
const openChromeExtBtn = document.getElementById('openChromeExtBtn');
const guideError = document.getElementById('guideError');

const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);
let detectedRemotes = [];
let currentBrowsePath = '';
let saveTimer = null;

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

function switchTab(isActivity) {
  viewActivity.classList.toggle('hidden', !isActivity);
  viewSettings.classList.toggle('hidden', isActivity);
  tabActivity.classList.toggle('active', isActivity);
  tabSettings.classList.toggle('active', !isActivity);
}

function updatePathPreview() {
  const base = (remoteInput.value || 'gdrive:YouTube Saved').trim().replace(/\/+$/, '');
  pathPreview.textContent = groupByChannelInput.checked
    ? `Example: ${joinRemote(base, 'Channel Name')}/video.mp4`
    : `Example: ${base}/video.mp4`;
}

function renderTools(tools = {}) {
  const items = [
    ['yt-dlp', tools.ytDlp],
    ['ffmpeg', tools.ffmpeg],
    ['rclone', tools.rclone],
  ];
  const allOk = items.every(([, ok]) => ok);
  sysBtn.textContent = allOk ? 'Engine: OK ▾' : 'Engine: Missing ▾';
  sysBtn.classList.toggle('warn', !allOk);
  installToolsBtn.classList.toggle('hidden', allOk);
  if (!allOk) {
    sysDrawer.classList.remove('hidden');
  }

  toolsRow.innerHTML = items
    .map(
      ([name, ok]) =>
        `<span class="${ok ? 'tool-ok' : 'tool-missing'}">${ok ? '✓' : '✗'} ${name}</span>`,
    )
    .join('');
}

function renderJobs(jobs = []) {
  const activeCount = jobs.filter((j) => ACTIVE_STATUSES.has(j.status)).length;
  tabActivity.textContent = activeCount > 0 ? `Activity (${activeCount})` : 'Activity';

  if (!jobs.length) {
    jobsList.innerHTML =
      '<p class="empty">No jobs yet. Click "Save to Cloud" on YouTube or paste a link above.</p>';
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

async function openDirectory(remotePath, allowFallback = true) {
  currentBrowsePath = remotePath;
  browserPathEl.textContent = remotePath;
  dirList.innerHTML = '<p class="empty">Loading folders...</p>';
  try {
    const dirs = await invoke('list_dirs', { path: remotePath });
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

async function persistSettings() {
  try {
    await invoke('save_app_config', {
      patch: {
        rcloneRemote: remoteInput.value.trim(),
        groupByChannel: groupByChannelInput.checked,
        quality: qualitySelect.value,
        cookiesBrowser: cookiesSelect.value,
      },
    });
    settingsMsg.textContent = '✓ Settings auto-saved';
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      settingsMsg.textContent = '';
    }, 2000);
  } catch (err) {
    settingsMsg.textContent = String(err);
  }
}

async function handleConnectDrive(triggerBtn) {
  connectHint.classList.remove('hidden');
  connectHint.textContent = 'Opening browser for Google Drive sign-in...';
  triggerBtn.disabled = true;
  try {
    await invoke('connect_gdrive', { remoteName: 'gdrive' });
    connectHint.textContent = 'Google Drive connected!';
    await loadState();
  } catch (err) {
    connectHint.textContent = `Failed: ${err}`;
  } finally {
    triggerBtn.disabled = false;
  }
}

async function loadState() {
  try {
    const data = await invoke('get_dashboard_state');
    renderTools(data.tools);
    detectedRemotes = data.remotes || [];

    const hasRemote = detectedRemotes.length > 0;
    setupBanner.classList.toggle('hidden', hasRemote);
    accountAvatar.classList.toggle('connected', hasRemote);
    remotesText.textContent = hasRemote
      ? `Connected (${detectedRemotes.join(', ')})`
      : 'Not connected yet';
    connectDriveBtn.textContent = hasRemote ? 'Reconnect' : 'Connect';

    if (data.config) {
      if (!remoteInput.matches(':focus')) remoteInput.value = data.config.rcloneRemote || '';
      groupByChannelInput.checked = Boolean(data.config.groupByChannel);
      qualitySelect.value = data.config.quality || 'best';
      cookiesSelect.value = data.config.cookiesBrowser || '';
      updatePathPreview();
    }

    const count = data.savedCount || 0;
    syncStatusText.textContent = data.lastSyncedAt
      ? `Cloud library: ${count} video${count === 1 ? '' : 's'} synced`
      : `Cloud library: ${count} video${count === 1 ? '' : 's'} (not synced yet)`;

    renderJobs(data.jobs);
  } catch (err) {
    connectHint.classList.remove('hidden');
    connectHint.textContent = String(err);
  }
}

tabActivity.addEventListener('click', () => switchTab(true));
tabSettings.addEventListener('click', () => switchTab(false));
sysBtn.addEventListener('click', () => sysDrawer.classList.toggle('hidden'));

bannerConnectBtn.addEventListener('click', () => handleConnectDrive(bannerConnectBtn));
connectDriveBtn.addEventListener('click', () => handleConnectDrive(connectDriveBtn));

syncDriveBtn.addEventListener('click', async () => {
  syncDriveBtn.disabled = true;
  syncDriveBtn.textContent = 'Syncing...';
  try {
    await invoke('sync_cloud_ids');
    await loadState();
  } catch (err) {
    syncStatusText.textContent = `Sync failed: ${err}`;
  } finally {
    syncDriveBtn.disabled = false;
    syncDriveBtn.textContent = '↻ Sync Drive';
  }
});

quickSaveForm.addEventListener('submit', async (e) => {
  e.preventDefault();
  quickSaveError.classList.add('hidden');
  const url = quickUrlInput.value.trim();
  if (!url) return;

  quickSaveBtn.disabled = true;
  try {
    await invoke('save_url', { url });
    quickUrlInput.value = '';
    await loadState();
  } catch (err) {
    quickSaveError.textContent = String(err);
    quickSaveError.classList.remove('hidden');
  } finally {
    quickSaveBtn.disabled = false;
  }
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

useDirBtn.addEventListener('click', async () => {
  remoteInput.value = currentBrowsePath;
  updatePathPreview();
  browserBox.classList.add('hidden');
  await persistSettings();
});

dirList.addEventListener('click', (e) => {
  const dirName = e.target.getAttribute('data-dir');
  if (dirName) openDirectory(joinRemote(currentBrowsePath, dirName));
});

createDirBtn.addEventListener('click', async () => {
  const name = newDirInput.value.trim();
  if (!name) return;
  try {
    await invoke('create_dir', { path: joinRemote(currentBrowsePath, name) });
    newDirInput.value = '';
    await openDirectory(currentBrowsePath);
  } catch (err) {
    dirList.innerHTML = `<p class="job-error">${escapeHtml(err)}</p>`;
  }
});

remoteInput.addEventListener('input', updatePathPreview);
remoteInput.addEventListener('change', persistSettings);
groupByChannelInput.addEventListener('change', () => {
  updatePathPreview();
  persistSettings();
});
qualitySelect.addEventListener('change', persistSettings);
cookiesSelect.addEventListener('change', persistSettings);

installToolsBtn.addEventListener('click', async () => {
  installToolsBtn.disabled = true;
  installToolsBtn.textContent = 'Downloading...';
  try {
    await invoke('download_missing_tools');
    await loadState();
  } catch (err) {
    connectHint.classList.remove('hidden');
    connectHint.textContent = `Tool download failed: ${err}`;
  } finally {
    installToolsBtn.disabled = false;
    installToolsBtn.textContent = 'Download Missing Tools';
  }
});

openExtFolderBtn.addEventListener('click', async () => {
  guideError.classList.add('hidden');
  try {
    await invoke('open_extension_folder');
  } catch (err) {
    guideError.textContent = String(err);
    guideError.classList.remove('hidden');
  }
});

openChromeExtBtn.addEventListener('click', async () => {
  guideError.classList.add('hidden');
  try {
    await invoke('open_chrome_extensions');
  } catch (err) {
    guideError.textContent = String(err);
    guideError.classList.remove('hidden');
  }
});

jobsList.addEventListener('click', async (e) => {
  const cancelId = e.target.getAttribute('data-cancel');
  const deleteId = e.target.getAttribute('data-delete');
  if (cancelId) {
    await invoke('cancel_job', { id: cancelId });
    await loadState();
  } else if (deleteId) {
    await invoke('delete_job', { id: deleteId });
    await loadState();
  }
});

guideBtn.addEventListener('click', () => guideModal.classList.remove('hidden'));
closeGuideBtn.addEventListener('click', () => guideModal.classList.add('hidden'));
guideModal.addEventListener('click', (e) => {
  if (e.target === guideModal) guideModal.classList.add('hidden');
});

loadState();
setInterval(loadState, 2000);
