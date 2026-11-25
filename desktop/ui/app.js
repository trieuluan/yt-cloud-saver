const { invoke } = window.__TAURI__.core;

const sysBtn = document.getElementById('sysBtn');
const sysDrawer = document.getElementById('sysDrawer');
const toolsRow = document.getElementById('toolsRow');
const installToolsBtn = document.getElementById('installToolsBtn');
const tabActivity = document.getElementById('tabActivity');
const tabSettings = document.getElementById('tabSettings');
const tabAi = document.getElementById('tabAi');
const viewActivity = document.getElementById('viewActivity');
const viewSettings = document.getElementById('viewSettings');
const viewAi = document.getElementById('viewAi');

const setupBanner = document.getElementById('setupBanner');
const bannerConnectBtn = document.getElementById('bannerConnectBtn');
const quickSaveForm = document.getElementById('quickSaveForm');
const quickUrlInput = document.getElementById('quickUrlInput');
const quickSaveBtn = document.getElementById('quickSaveBtn');
const quickSaveError = document.getElementById('quickSaveError');
const syncStatusText = document.getElementById('syncStatusText');
const syncDriveBtn = document.getElementById('syncDriveBtn');
const backupSubsBtn = document.getElementById('backupSubsBtn');
const restoreSubsBtn = document.getElementById('restoreSubsBtn');
const fltBtns = document.querySelectorAll('.flt-btn');
const activityHint = document.getElementById('activityHint');
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

const provBtns = document.querySelectorAll('.prov-btn');
const aiBaseUrlWrap = document.getElementById('aiBaseUrlWrap');
const aiApiKeyWrap = document.getElementById('aiApiKeyWrap');
const aiBaseUrlInput = document.getElementById('aiBaseUrlInput');
const aiApiKeyInput = document.getElementById('aiApiKeyInput');
const aiModelInput = document.getElementById('aiModelInput');
const aiModelSelect = document.getElementById('aiModelSelect');
const fetchModelsBtn = document.getElementById('fetchModelsBtn');
const testAiBtn = document.getElementById('testAiBtn');
const aiTestMsg = document.getElementById('aiTestMsg');
const targetLangSelect = document.getElementById('targetLangSelect');
const whisperFallbackInput = document.getElementById('whisperFallbackInput');
const preferWhisperInput = document.getElementById('preferWhisperInput');
const whisperCliStatus = document.getElementById('whisperCliStatus');
const whisperCliInstallBtn = document.getElementById('whisperCliInstallBtn');
const whisperCliRemoveBtn = document.getElementById('whisperCliRemoveBtn');
const whisperModelSelect = document.getElementById('whisperModelSelect');
const whisperModelStatus = document.getElementById('whisperModelStatus');
const whisperModelDlBtn = document.getElementById('whisperModelDlBtn');
const whisperModelRmBtn = document.getElementById('whisperModelRmBtn');
const whisperMsg = document.getElementById('whisperMsg');
const embedSubtitlesInput = document.getElementById('embedSubtitlesInput');
const aiSettingsMsg = document.getElementById('aiSettingsMsg');

const guideBtn = document.getElementById('guideBtn');
const guideModal = document.getElementById('guideModal');
const closeGuideBtn = document.getElementById('closeGuideBtn');
const supportBtn = document.getElementById('supportBtn');
const supportModal = document.getElementById('supportModal');
const closeSupportBtn = document.getElementById('closeSupportBtn');
const openExtFolderBtn = document.getElementById('openExtFolderBtn');
const openChromeExtBtn = document.getElementById('openChromeExtBtn');
const guideError = document.getElementById('guideError');

const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);
let detectedRemotes = [];
let configHydrated = false;
let currentBrowsePath = '';
let saveTimer = null;
let downloadedWhisperModels = [];
let activityFilter = 'all';
let latestJobs = [];
let latestTranslations = [];

const WHISPER_SIZES = {
  tiny: '75 MB',
  base: '142 MB',
  small: '466 MB',
  medium: '1.5 GB',
  'large-v3-turbo-q5_0': '574 MB',
  'large-v3-turbo': '1.62 GB',
};

function renderWhisperUI(whisper) {
  if (whisper) {
    downloadedWhisperModels = whisper.downloadedModels || [];
    const cliOk = Boolean(whisper.cliInstalled);
    whisperCliStatus.textContent = cliOk ? '✓ Installed' : 'Not installed';
    whisperCliStatus.className = cliOk ? 'tool-ok' : 'muted';
    whisperCliInstallBtn.classList.toggle('hidden', cliOk);
    whisperCliRemoveBtn.classList.toggle('hidden', !cliOk);
  }
  const m = whisperModelSelect.value || 'small';
  const modelOk = downloadedWhisperModels.includes(m);
  whisperModelStatus.textContent = modelOk
    ? `✓ ggml-${m}.bin (${WHISPER_SIZES[m] || ''})`
    : `ggml-${m}.bin · Not downloaded`;
  whisperModelStatus.className = modelOk ? 'tool-ok' : 'muted';
  whisperModelDlBtn.classList.toggle('hidden', modelOk);
  whisperModelRmBtn.classList.toggle('hidden', !modelOk);
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

let currentAiProvider = 'gemini';
let aiProfiles = {
  gemini: { baseUrl: '', apiKey: '', model: 'gemini-3.8-flash' },
  openai: { baseUrl: 'https://api.openai.com/v1', apiKey: '', model: 'gpt-4o-mini' },
  ollama: { baseUrl: 'http://localhost:11434/v1', apiKey: '', model: 'qwen2.5:14b' },
};
const aiModelLists = { gemini: [], openai: [], ollama: [] };

function switchTab(tab) {
  viewActivity.classList.toggle('hidden', tab !== 'activity');
  viewSettings.classList.toggle('hidden', tab !== 'settings');
  viewAi.classList.toggle('hidden', tab !== 'ai');
  tabActivity.classList.toggle('active', tab === 'activity');
  tabSettings.classList.toggle('active', tab === 'settings');
  tabAi.classList.toggle('active', tab === 'ai');
}

function renderProviderModelsDropdown(provider) {
  const list = aiModelLists[provider] || [];
  const current = aiModelInput.value.trim();
  if (!list.length) {
    aiModelSelect.classList.add('hidden');
    aiModelSelect.innerHTML = '';
    return;
  }
  aiModelSelect.innerHTML =
    `<option value="">-- Select from ${list.length} models --</option>` +
    list
      .map((m) => `<option value="${escapeHtml(m)}"${m === current ? ' selected' : ''}>${escapeHtml(m)}</option>`)
      .join('');
  aiModelSelect.classList.remove('hidden');
}

function saveCurrentProviderInputs() {
  aiProfiles[currentAiProvider] = {
    baseUrl: aiBaseUrlInput.value.trim(),
    apiKey: aiApiKeyInput.value.trim(),
    model: aiModelInput.value.trim(),
  };
}

function renderProviderFields(provider) {
  currentAiProvider = provider || 'gemini';
  const prof = aiProfiles[currentAiProvider] || { baseUrl: '', apiKey: '', model: '' };
  provBtns.forEach((b) => b.classList.toggle('active', b.dataset.prov === currentAiProvider));
  aiBaseUrlWrap.classList.toggle('hidden', currentAiProvider === 'gemini');
  aiApiKeyWrap.classList.toggle('hidden', currentAiProvider === 'ollama');
  aiBaseUrlInput.value = prof.baseUrl || '';
  aiApiKeyInput.value = prof.apiKey || '';
  aiModelInput.value = prof.model || '';
  renderProviderModelsDropdown(currentAiProvider);
  aiTestMsg.classList.add('hidden');
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

function renderJobs(jobs = [], translations = []) {
  latestJobs = jobs;
  latestTranslations = translations;
  const activeCount = jobs.filter((j) => ACTIVE_STATUSES.has(j.status)).length;
  tabActivity.textContent = activeCount > 0 ? `Activity (${activeCount})` : 'Activity';

  const filteredJobs = jobs.filter((j) => {
    const kind = j.kind || 'cloud';
    return activityFilter === 'all' || kind === activityFilter;
  });

  const showTranslations = activityFilter === 'all' || activityFilter === 'translate';
  const cachedItems = showTranslations ? translations : [];

  if (!filteredJobs.length && !cachedItems.length) {
    jobsList.innerHTML =
      '<p class="empty">No activity yet. Click "Save to Cloud" or "CC" on YouTube.</p>';
    return;
  }

  const jobHtml = filteredJobs
    .map((job) => {
      const kind = job.kind || 'cloud';
      const isActive = ACTIVE_STATUSES.has(job.status);
      const isRetryable = job.status === 'error' || job.status === 'cancelled';
      const pct = job.status === 'done' ? 100 : job.progress || 0;
      const actionBtn = isActive
        ? `<button class="btn-ghost" data-cancel="${escapeHtml(job.id)}">Cancel</button>`
        : isRetryable
          ? `<div class="btn-group"><button class="btn-primary btn-xs" data-retry="${escapeHtml(job.id)}">Resume</button><button class="btn-ghost" data-delete="${escapeHtml(job.id)}">Remove</button></div>`
          : `<button class="btn-ghost" data-delete="${escapeHtml(job.id)}">Remove</button>`;
      const badge =
        kind === 'translate'
          ? '<span class="kind-pill kind-translate">Translate</span>'
          : '<span class="kind-pill kind-cloud">Cloud</span>';
      const barCls = job.status === 'done' ? 'done' : kind === 'translate' ? 'translate' : '';

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

  const openExportFile = jobsList.querySelector('.export-dropdown.open')?.getAttribute('data-file') || null;

  const subHtml = cachedItems
    .map(
      (sub) => {
        const isOpen = openExportFile === sub.fileName ? ' open' : '';
        return `
      <div class="job-item">
        <div class="job-top">
          <div class="job-title"><span class="kind-pill kind-sub">Sub ✓</span>${escapeHtml(sub.title)}</div>
          <div class="btn-group">
            <div class="export-dropdown${isOpen}" data-file="${escapeHtml(sub.fileName)}">
              <button type="button" class="btn-ghost btn-xs" data-export-toggle="${escapeHtml(sub.fileName)}">Export .srt ▾</button>
              <div class="export-menu">
                <button type="button" class="export-option" data-export-sub="${escapeHtml(sub.fileName)}" data-export-mode="translated">Translated (${escapeHtml(sub.targetLang)})</button>
                <button type="button" class="export-option" data-export-sub="${escapeHtml(sub.fileName)}" data-export-mode="original">Original (sub gốc)</button>
                <button type="button" class="export-option" data-export-sub="${escapeHtml(sub.fileName)}" data-export-mode="bilingual">Bilingual (song ngữ)</button>
              </div>
            </div>
            <button class="btn-danger-ghost btn-xs" data-delete-sub="${escapeHtml(sub.fileName)}">Delete</button>
          </div>
        </div>
        <div class="job-meta">${escapeHtml(sub.targetLang)} · ${escapeHtml(sub.source)} · ${sub.lineCount} lines</div>
      </div>
    `;
      },
    )
    .join('');

  jobsList.innerHTML = jobHtml + subHtml;
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
    saveCurrentProviderInputs();
    const active = aiProfiles[currentAiProvider];
    await invoke('save_app_config', {
      patch: {
        rcloneRemote: remoteInput.value.trim(),
        groupByChannel: groupByChannelInput.checked,
        quality: qualitySelect.value,
        cookiesBrowser: cookiesSelect.value,
        aiProvider: currentAiProvider,
        aiBaseUrl: active.baseUrl,
        aiApiKey: active.apiKey,
        aiModel: active.model,
        aiProfiles,
        targetLang: targetLangSelect.value,
        useWhisperFallback: whisperFallbackInput.checked,
        whisperModel: whisperModelSelect.value,
        preferWhisperOverAutoCc: preferWhisperInput.checked,
        embedSubtitles: embedSubtitlesInput.checked,
      },
    });
    settingsMsg.textContent = '✓ Settings auto-saved';
    aiSettingsMsg.textContent = '✓ Settings auto-saved';
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      settingsMsg.textContent = '';
      aiSettingsMsg.textContent = '';
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
    backupSubsBtn.classList.toggle('hidden', !hasRemote);
    restoreSubsBtn.classList.toggle('hidden', !hasRemote);
    syncDriveBtn.classList.toggle('hidden', !hasRemote);
    remotesText.textContent = hasRemote
      ? `Connected (${detectedRemotes.join(', ')})`
      : 'Not connected yet';
    connectDriveBtn.textContent = hasRemote ? 'Reconnect' : 'Connect';

    if (data.config && !configHydrated) {
      configHydrated = true;
      remoteInput.value = data.config.rcloneRemote || '';
      groupByChannelInput.checked = Boolean(data.config.groupByChannel);
      qualitySelect.value = data.config.quality || 'best';
      cookiesSelect.value = data.config.cookiesBrowser || '';
      if (data.config.aiProfiles) {
        aiProfiles = { ...aiProfiles, ...data.config.aiProfiles };
      }
      renderProviderFields(data.config.aiProvider || 'gemini');
      targetLangSelect.value = data.config.targetLang || 'Vietnamese';
      whisperFallbackInput.checked = Boolean(data.config.useWhisperFallback);
      whisperModelSelect.value = data.config.whisperModel || 'small';
      preferWhisperInput.checked = Boolean(data.config.preferWhisperOverAutoCc);
      embedSubtitlesInput.checked = Boolean(data.config.embedSubtitles);
      updatePathPreview();
    }
    renderWhisperUI(data.whisper);

    const count = data.savedCount || 0;
    syncStatusText.textContent = data.lastSyncedAt
      ? `Cloud library: ${count} video${count === 1 ? '' : 's'} synced`
      : `Cloud library: ${count} video${count === 1 ? '' : 's'} (not synced yet)`;

    renderJobs(data.jobs, data.translations || []);
  } catch (err) {
    connectHint.classList.remove('hidden');
    connectHint.textContent = String(err);
  }
}

tabActivity.addEventListener('click', () => switchTab('activity'));
tabSettings.addEventListener('click', () => switchTab('settings'));
tabAi.addEventListener('click', () => switchTab('ai'));
sysBtn.addEventListener('click', () => sysDrawer.classList.toggle('hidden'));

provBtns.forEach((btn) => {
  btn.addEventListener('click', () => {
    saveCurrentProviderInputs();
    const prov = btn.dataset.prov;
    renderProviderFields(prov);
    persistSettings();
  });
});

[aiBaseUrlInput, aiApiKeyInput, aiModelInput].forEach((el) => el.addEventListener('change', persistSettings));
[targetLangSelect, whisperFallbackInput, preferWhisperInput, embedSubtitlesInput].forEach((el) => el.addEventListener('change', persistSettings));

whisperModelSelect.addEventListener('change', () => {
  renderWhisperUI();
  persistSettings();
});

whisperCliInstallBtn.addEventListener('click', async () => {
  whisperMsg.classList.add('hidden');
  whisperCliInstallBtn.disabled = true;
  whisperCliInstallBtn.textContent = 'Installing...';
  try {
    const status = await invoke('install_whisper_cli_cmd');
    renderWhisperUI(status);
  } catch (err) {
    whisperMsg.textContent = String(err);
    whisperMsg.classList.remove('hidden');
  } finally {
    whisperCliInstallBtn.disabled = false;
    whisperCliInstallBtn.textContent = 'Install';
  }
});

whisperCliRemoveBtn.addEventListener('click', async () => {
  whisperMsg.classList.add('hidden');
  whisperCliRemoveBtn.disabled = true;
  try {
    const status = await invoke('remove_whisper_cli_cmd');
    renderWhisperUI(status);
  } catch (err) {
    whisperMsg.textContent = String(err);
    whisperMsg.classList.remove('hidden');
  } finally {
    whisperCliRemoveBtn.disabled = false;
  }
});

whisperModelDlBtn.addEventListener('click', async () => {
  const modelName = whisperModelSelect.value;
  whisperMsg.classList.add('hidden');
  whisperModelDlBtn.disabled = true;
  whisperModelDlBtn.textContent = 'Downloading...';
  try {
    const status = await invoke('download_whisper_model_cmd', { modelName });
    renderWhisperUI(status);
  } catch (err) {
    whisperMsg.textContent = String(err);
    whisperMsg.classList.remove('hidden');
  } finally {
    whisperModelDlBtn.disabled = false;
    whisperModelDlBtn.textContent = 'Download';
  }
});

whisperModelRmBtn.addEventListener('click', async () => {
  const modelName = whisperModelSelect.value;
  whisperMsg.classList.add('hidden');
  whisperModelRmBtn.disabled = true;
  try {
    const status = await invoke('remove_whisper_model_cmd', { modelName });
    renderWhisperUI(status);
  } catch (err) {
    whisperMsg.textContent = String(err);
    whisperMsg.classList.remove('hidden');
  } finally {
    whisperModelRmBtn.disabled = false;
  }
});

fetchModelsBtn.addEventListener('click', async () => {
  await persistSettings();
  aiTestMsg.classList.remove('hidden');
  aiTestMsg.textContent = 'Fetching available models...';
  fetchModelsBtn.disabled = true;
  try {
    const models = await invoke('list_ai_models_cmd');
    aiModelLists[currentAiProvider] = models || [];
    renderProviderModelsDropdown(currentAiProvider);
    aiTestMsg.textContent = `Loaded ${models.length} models — pick one from the dropdown below.`;
  } catch (err) {
    aiTestMsg.textContent = `✗ ${err}`;
  } finally {
    fetchModelsBtn.disabled = false;
  }
});

aiModelSelect.addEventListener('change', () => {
  if (aiModelSelect.value) {
    aiModelInput.value = aiModelSelect.value;
    persistSettings();
  }
});

testAiBtn.addEventListener('click', async () => {
  await persistSettings();
  aiTestMsg.classList.remove('hidden');
  aiTestMsg.textContent = 'Testing connection...';
  testAiBtn.disabled = true;
  try {
    const res = await invoke('test_ai_connection');
    aiTestMsg.textContent = `✓ Connected (${res})`;
  } catch (err) {
    aiTestMsg.textContent = `✗ ${err}`;
  } finally {
    testAiBtn.disabled = false;
  }
});

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

backupSubsBtn.addEventListener('click', async () => {
  backupSubsBtn.disabled = true;
  backupSubsBtn.textContent = 'Backing up...';
  try {
    const count = await invoke('backup_subtitles_cloud');
    activityHint.classList.remove('hidden');
    activityHint.textContent = `Backed up ${count} subtitle file(s) to Google Drive (_ytcs_subtitles).`;
  } catch (err) {
    activityHint.classList.remove('hidden');
    activityHint.textContent = `Backup failed: ${err}`;
  } finally {
    backupSubsBtn.disabled = false;
    backupSubsBtn.textContent = '↑ Backup Subs';
  }
});

restoreSubsBtn.addEventListener('click', async () => {
  restoreSubsBtn.disabled = true;
  restoreSubsBtn.textContent = 'Restoring...';
  try {
    const total = await invoke('restore_subtitles_cloud');
    activityHint.classList.remove('hidden');
    activityHint.textContent = `Restored subtitles from Drive (${total} cached locally).`;
    await loadState();
  } catch (err) {
    activityHint.classList.remove('hidden');
    activityHint.textContent = `Restore failed: ${err}`;
  } finally {
    restoreSubsBtn.disabled = false;
    restoreSubsBtn.textContent = '↓ Restore Subs';
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

fltBtns.forEach((btn) => {
  btn.addEventListener('click', () => {
    activityFilter = btn.dataset.filter;
    fltBtns.forEach((b) => b.classList.toggle('active', b === btn));
    renderJobs(latestJobs, latestTranslations);
  });
});

jobsList.addEventListener('click', async (e) => {
  const target = e.target;
  const toggleFile = target.getAttribute('data-export-toggle');
  if (toggleFile) {
    e.stopPropagation();
    const current = target.closest('.export-dropdown');
    const wasOpen = current?.classList.contains('open');
    jobsList.querySelectorAll('.export-dropdown.open').forEach((el) => el.classList.remove('open'));
    if (current && !wasOpen) current.classList.add('open');
    return;
  }

  const cancelId = target.getAttribute('data-cancel');
  const retryId = target.getAttribute('data-retry');
  const deleteId = target.getAttribute('data-delete');
  const exportSub = target.getAttribute('data-export-sub');
  const exportMode = target.getAttribute('data-export-mode') || 'translated';
  const deleteSub = target.getAttribute('data-delete-sub');
  if (cancelId) {
    await invoke('cancel_job', { id: cancelId });
    await loadState();
  } else if (retryId) {
    await invoke('retry_job', { id: retryId });
    await loadState();
  } else if (deleteId) {
    await invoke('delete_job', { id: deleteId });
    await loadState();
  } else if (exportSub) {
    jobsList.querySelectorAll('.export-dropdown.open').forEach((el) => el.classList.remove('open'));
    try {
      const savedPath = await invoke('export_subtitle_srt', { fileName: exportSub, mode: exportMode });
      activityHint.classList.remove('hidden');
      activityHint.textContent = `Exported: ${savedPath}`;
    } catch (err) {
      activityHint.classList.remove('hidden');
      activityHint.textContent = `Export failed: ${err}`;
    }
  } else if (deleteSub) {
    await invoke('delete_subtitle_cache', { fileName: deleteSub });
    await loadState();
  }
});

document.addEventListener('click', (e) => {
  if (!e.target.closest('.export-dropdown')) {
    jobsList.querySelectorAll('.export-dropdown.open').forEach((el) => el.classList.remove('open'));
  }
});

guideBtn.addEventListener('click', () => guideModal.classList.remove('hidden'));
closeGuideBtn.addEventListener('click', () => guideModal.classList.add('hidden'));
guideModal.addEventListener('click', (e) => {
  if (e.target === guideModal) guideModal.classList.add('hidden');
});

supportBtn.addEventListener('click', () => supportModal.classList.remove('hidden'));
closeSupportBtn.addEventListener('click', () => supportModal.classList.add('hidden'));
supportModal.addEventListener('click', (e) => {
  if (e.target === supportModal) supportModal.classList.add('hidden');
});

loadState();
setInterval(loadState, 2000);
