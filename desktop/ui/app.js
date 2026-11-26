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
const whisperLangSelect = document.getElementById('whisperLangSelect');
const whisperModeLocalBtn = document.getElementById('whisperModeLocalBtn');
const whisperModeCloudBtn = document.getElementById('whisperModeCloudBtn');
const whisperLocalPanel = document.getElementById('whisperLocalPanel');
const whisperCloudPanel = document.getElementById('whisperCloudPanel');
const whisperCloudProvider = document.getElementById('whisperCloudProvider');
const whisperCloudKeyInput = document.getElementById('whisperCloudKeyInput');
const whisperCloudModelInput = document.getElementById('whisperCloudModelInput');
const whisperCloudBaseUrlInput = document.getElementById('whisperCloudBaseUrlInput');
const testWhisperCloudBtn = document.getElementById('testWhisperCloudBtn');
const whisperCloudTestMsg = document.getElementById('whisperCloudTestMsg');
const whisperModelStatus = document.getElementById('whisperModelStatus');
const whisperModelDlBtn = document.getElementById('whisperModelDlBtn');
const whisperModelRmBtn = document.getElementById('whisperModelRmBtn');
const whisperMsg = document.getElementById('whisperMsg');
const vieneuSidecarBadge = document.getElementById('vieneuSidecarBadge');
const ttsModeEdgeBtn = document.getElementById('ttsModeEdgeBtn');
const ttsModeLocalBtn = document.getElementById('ttsModeLocalBtn');
const ttsEdgePanel = document.getElementById('ttsEdgePanel');
const ttsLocalPanel = document.getElementById('ttsLocalPanel');
const vieneuRuntimeStatus = document.getElementById('vieneuRuntimeStatus');
const vieneuInstallBtn = document.getElementById('vieneuInstallBtn');
const vieneuRemoveBtn = document.getElementById('vieneuRemoveBtn');
const vieneuVariantList = document.getElementById('vieneuVariantList');
const vieneuVoiceSelect = document.getElementById('vieneuVoiceSelect');
const vieneuCustomOptGroup = document.getElementById('vieneuCustomOptGroup');
const vieneuTestVoiceBtn = document.getElementById('vieneuTestVoiceBtn');
const vieneuDelVoiceBtn = document.getElementById('vieneuDelVoiceBtn');
const vieneuRefAudioRow = document.getElementById('vieneuRefAudioRow');
const vieneuCustomNameInput = document.getElementById('vieneuCustomNameInput');
const vieneuRefAudioInput = document.getElementById('vieneuRefAudioInput');
const vieneuBrowseAudioBtn = document.getElementById('vieneuBrowseAudioBtn');
const vieneuSaveCustomBtn = document.getElementById('vieneuSaveCustomBtn');
const vieneuCancelCustomBtn = document.getElementById('vieneuCancelCustomBtn');
const vieneuMsg = document.getElementById('vieneuMsg');
const embedSubtitlesInput = document.getElementById('embedSubtitlesInput');
const aiSettingsMsg = document.getElementById('aiSettingsMsg');

const guideBtn = document.getElementById('guideBtn');
const guideModal = document.getElementById('guideModal');
const closeGuideBtn = document.getElementById('closeGuideBtn');
const supportBtn = document.getElementById('supportBtn');
const appVersionBadge = document.getElementById('appVersionBadge');
const supportModal = document.getElementById('supportModal');
const closeSupportBtn = document.getElementById('closeSupportBtn');
const checkUpdateBtn = document.getElementById('checkUpdateBtn');
const updateBanner = document.getElementById('updateBanner');
const updateTitle = document.getElementById('updateTitle');
const updateSub = document.getElementById('updateSub');
const installUpdateBtn = document.getElementById('installUpdateBtn');
const openExtFolderBtn = document.getElementById('openExtFolderBtn');
const openChromeExtBtn = document.getElementById('openChromeExtBtn');
const guideError = document.getElementById('guideError');

const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);
let detectedRemotes = [];
let configHydrated = false;
let currentBrowsePath = '';
let saveTimer = null;
let downloadedWhisperModels = [];
let currentTtsMode = 'edge';
let activeVieneuModel = 'v3turbo';
let vieneuRuntimeInstalled = false;
let downloadedVieneuModels = [];
let downloadingVieneuModel = '';
let customVoices = [];
let lastValidVieneuVoice = 'Ngọc Huyền';
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

const VIENEU_VARIANTS = [
  {
    id: 'v3turbo',
    name: 'VieNeu-TTS v3 Turbo',
    size: '~580 MB · 48kHz',
    badge: 'Khuyên dùng · Nhanh nhất',
    desc: 'ONNX CPU ~300ms/câu, hỗ trợ tốt tiếng Việt & Anh xen kẽ',
  },
  {
    id: 'v3nano',
    name: 'VieNeu-TTS v3 Nano',
    size: '~180 MB · 24kHz',
    badge: 'Siêu nhẹ CPU',
    desc: '48M-param tối ưu cho máy cấu hình yếu hoặc tiết kiệm RAM',
  },
];

function renderTtsMode(mode) {
  currentTtsMode = mode === 'vieneu' ? 'vieneu' : 'edge';
  const isLocal = currentTtsMode === 'vieneu';
  ttsModeEdgeBtn.classList.toggle('active', !isLocal);
  ttsModeLocalBtn.classList.toggle('active', isLocal);
  ttsEdgePanel.classList.toggle('hidden', isLocal);
  ttsLocalPanel.classList.toggle('hidden', !isLocal);
}

function renderCustomVoicesOptions() {
  if (!vieneuCustomOptGroup) return;
  vieneuCustomOptGroup.innerHTML = customVoices
    .map((v) => `<option value="custom:${escapeHtml(v.id)}">🎙 ${escapeHtml(v.name)}</option>`)
    .join('');
  vieneuCustomOptGroup.style.display = customVoices.length ? '' : 'none';
}

function renderVieneuUI(vieneu) {
  if (vieneu) {
    vieneuRuntimeInstalled = Boolean(vieneu.runtimeInstalled);
    downloadedVieneuModels = vieneu.downloadedModels || [];
    vieneuRuntimeStatus.textContent = vieneuRuntimeInstalled
      ? '✓ Installed · Sẵn sàng chạy local'
      : 'Not installed (~650 MB venv)';
    vieneuRuntimeStatus.className = vieneuRuntimeInstalled ? 'tool-ok' : 'muted';
    vieneuInstallBtn.classList.toggle('hidden', vieneuRuntimeInstalled);
    vieneuRemoveBtn.classList.toggle('hidden', !vieneuRuntimeInstalled);
    if (vieneuSidecarBadge) {
      vieneuSidecarBadge.textContent = vieneu.sidecarRunning
        ? `Sidecar: Ready (${activeVieneuModel}) · Stop`
        : 'Sidecar: Off';
      vieneuSidecarBadge.className = vieneu.sidecarRunning ? 'tool-ok' : 'muted';
      vieneuSidecarBadge.style.cursor = vieneu.sidecarRunning ? 'pointer' : 'default';
      vieneuSidecarBadge.title = vieneu.sidecarRunning
        ? 'Bấm để dừng Sidecar giải phóng RAM (hoặc tự tắt sau 3 phút không dùng)'
        : 'Tự bật khi lồng tiếng VieNeu-TTS';
    }
  }
  vieneuVariantList.innerHTML = VIENEU_VARIANTS.map((v) => {
    const isDl = downloadedVieneuModels.includes(v.id);
    const isBusy = downloadingVieneuModel === v.id;
    const isActive = isDl && activeVieneuModel === v.id;
    let btns = '';
    if (isBusy) {
      btns = '<span class="muted">Downloading...</span>';
    } else if (isDl) {
      btns = `
        ${isActive ? '<span class="variant-tag rec">Đang dùng</span>' : `<button class="btn-ghost btn-xs" type="button" data-vn-use="${v.id}">Use</button>`}
        <button class="btn-danger-ghost btn-xs" type="button" data-vn-rm="${v.id}">Remove</button>
      `;
    } else {
      btns = `<button class="btn-primary btn-xs" type="button" data-vn-dl="${v.id}" ${vieneuRuntimeInstalled ? '' : 'disabled title="Cài Runtime trước"'}>Download</button>`;
    }
    return `
      <div class="variant-item ${isActive ? 'active-model' : ''}">
        <div style="min-width:0;">
          <div>
            <span class="sub-title">${escapeHtml(v.name)}</span>
            <span class="variant-tag">${escapeHtml(v.size)}</span>
            <span class="variant-tag rec">${escapeHtml(v.badge)}</span>
          </div>
          <div class="muted" style="font-size:11px;margin-top:2px;">${escapeHtml(v.desc)}</div>
        </div>
        <div class="btn-group">${btns}</div>
      </div>
    `;
  }).join('');
  const isAddingCustom = vieneuVoiceSelect.value === '__add_custom__';
  const isCustomSelected = vieneuVoiceSelect.value.startsWith('custom:');
  vieneuRefAudioRow.classList.toggle('hidden', !isAddingCustom);
  if (vieneuDelVoiceBtn) {
    vieneuDelVoiceBtn.classList.toggle('hidden', !isCustomSelected);
  }
  if (vieneuTestVoiceBtn) {
    const canTest =
      vieneuRuntimeInstalled &&
      downloadedVieneuModels.includes(activeVieneuModel) &&
      !isAddingCustom;
    vieneuTestVoiceBtn.disabled = !canTest;
    vieneuTestVoiceBtn.title = canTest
      ? 'Nghe thử giọng đang chọn'
      : 'Cần cài Runtime và tải ít nhất 1 Model để nghe thử';
  }
}

let currentWhisperMode = 'local';

function renderWhisperMode(mode) {
  currentWhisperMode = mode === 'cloud' ? 'cloud' : 'local';
  const isCloud = currentWhisperMode === 'cloud';
  whisperModeLocalBtn.classList.toggle('active', !isCloud);
  whisperModeCloudBtn.classList.toggle('active', isCloud);
  whisperLocalPanel.classList.toggle('hidden', isCloud);
  whisperCloudPanel.classList.toggle('hidden', !isCloud);
}

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
      const barCls = [
        job.status === 'done' ? 'done' : isRetryable ? 'error' : kind === 'translate' ? 'translate' : '',
        isActive ? 'active' : '',
      ]
        .filter(Boolean)
        .join(' ');
      const visualPct = isActive ? Math.max(pct, 8) : pct;
      const pulseDot = isActive
        ? `<span class="pulse-dot${kind === 'translate' ? ' translate' : ''}"></span>`
        : '';

      return `
        <div class="job-item">
          <div class="job-top">
            <div class="job-title">${badge}${escapeHtml(job.title || job.url)}</div>
            ${actionBtn}
          </div>
          <div class="job-meta">${pulseDot}${escapeHtml(job.stageText || job.status)} · ${escapeHtml(job.remote || '')}</div>
          ${job.error ? `<div class="job-error">${escapeHtml(job.error)}</div>` : ''}
          <div class="bar"><div class="bar-fill ${barCls}" style="width:${visualPct}%"></div></div>
          ${
            job.sizeText || job.speedText
              ? `<div class="job-stats"><span class="size-val">${escapeHtml(job.sizeText || '')}</span><span>${escapeHtml(job.speedText || '')}</span></div>`
              : ''
          }
        </div>
      `;
    })
    .join('');

  const openDropdownEl = jobsList.querySelector('.export-dropdown.open');
  const openDropdownKind = openDropdownEl?.getAttribute('data-kind') || null;
  const openDropdownFile = openDropdownEl?.getAttribute('data-file') || null;

  const subHtml = cachedItems
    .map(
      (sub) => {
        const isRerunOpen = openDropdownKind === 'rerun' && openDropdownFile === sub.fileName ? ' open' : '';
        const isExportOpen = openDropdownKind === 'export' && openDropdownFile === sub.fileName ? ' open' : '';
        return `
      <div class="job-item">
        <div class="job-title"><span class="kind-pill kind-sub">Sub ✓</span>${escapeHtml(sub.title)}</div>
        <div class="sub-footer">
          <div class="job-meta">${escapeHtml(sub.targetLang)} · ${escapeHtml(sub.source)} · ${sub.lineCount} lines</div>
          <div class="btn-group">
            <div class="export-dropdown${isRerunOpen}" data-kind="rerun" data-file="${escapeHtml(sub.fileName)}">
              <button type="button" class="btn-accent-ghost btn-xs" data-rerun-toggle="${escapeHtml(sub.fileName)}" title="Re-translate existing text or re-run Whisper from audio">↻ Re-run ▾</button>
              <div class="export-menu rerun-menu">
                <button type="button" class="export-option" data-retranslate-sub="${escapeHtml(sub.fileName)}" title="Keep cached transcription/CC and only re-translate lines">
                  <span class="menu-item-title" style="color:#d8b4fe;">↻ Re-translate</span>
                  <span class="menu-item-desc">Keep original text · AI translates again (fast)</span>
                </button>
                <button type="button" class="export-option" data-rewhisper-sub="${escapeHtml(sub.fileName)}" title="Delete cached subtitles, download audio, re-run Whisper & translate">
                  <span class="menu-item-title" style="color:#7dd3fc;">🎙 Re-whisper + Translate</span>
                  <span class="menu-item-desc">Re-download audio · Whisper again from scratch</span>
                </button>
              </div>
            </div>
            <div class="export-dropdown${isExportOpen}" data-kind="export" data-file="${escapeHtml(sub.fileName)}">
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
  if (!configHydrated) return;
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
        whisperLanguage: whisperLangSelect.value,
        whisperMode: currentWhisperMode,
        whisperCloudProvider: whisperCloudProvider.value,
        whisperCloudBaseUrl: whisperCloudBaseUrlInput.value.trim(),
        whisperCloudApiKey: whisperCloudKeyInput.value.trim(),
        whisperCloudModel: whisperCloudModelInput.value.trim(),
        preferWhisperOverAutoCc: preferWhisperInput.checked,
        embedSubtitles: embedSubtitlesInput.checked,
        ttsMode: currentTtsMode,
        vieneuModel: activeVieneuModel,
        vieneuVoice:
          vieneuVoiceSelect.value === '__add_custom__'
            ? lastValidVieneuVoice
            : vieneuVoiceSelect.value,
        vieneuRefAudio: vieneuRefAudioInput.value.trim(),
        customVoices,
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
    if (data.appVersion && appVersionBadge) {
      appVersionBadge.textContent = `v${data.appVersion}`;
    }
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
      whisperLangSelect.value = data.config.whisperLanguage || 'auto';
      renderWhisperMode(data.config.whisperMode || 'local');
      whisperCloudProvider.value = data.config.whisperCloudProvider || 'groq';
      whisperCloudBaseUrlInput.value = data.config.whisperCloudBaseUrl || 'https://api.groq.com/openai/v1';
      whisperCloudKeyInput.value = data.config.whisperCloudApiKey || '';
      whisperCloudModelInput.value = data.config.whisperCloudModel || 'whisper-large-v3-turbo';
      preferWhisperInput.checked = Boolean(data.config.preferWhisperOverAutoCc);
      embedSubtitlesInput.checked = Boolean(data.config.embedSubtitles);
      renderTtsMode(data.config.ttsMode || 'edge');
      activeVieneuModel = data.config.vieneuModel || 'v3turbo';
      customVoices = Array.isArray(data.config.customVoices) ? data.config.customVoices : [];
      renderCustomVoicesOptions();
      lastValidVieneuVoice = data.config.vieneuVoice || 'Ngọc Huyền';
      vieneuVoiceSelect.value = lastValidVieneuVoice;
      if (!vieneuVoiceSelect.value) {
        lastValidVieneuVoice = 'Ngọc Huyền';
        vieneuVoiceSelect.value = 'Ngọc Huyền';
      }
      vieneuRefAudioInput.value = data.config.vieneuRefAudio || '';
      updatePathPreview();
    }
    renderWhisperUI(data.whisper);
    renderVieneuUI(data.vieneu);

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
[targetLangSelect, whisperLangSelect, whisperFallbackInput, preferWhisperInput, embedSubtitlesInput].forEach((el) => el.addEventListener('change', persistSettings));
[whisperCloudKeyInput, whisperCloudModelInput, whisperCloudBaseUrlInput].forEach((el) => el.addEventListener('change', persistSettings));

whisperModeLocalBtn.addEventListener('click', () => {
  renderWhisperMode('local');
  persistSettings();
});

whisperModeCloudBtn.addEventListener('click', () => {
  renderWhisperMode('cloud');
  persistSettings();
});

whisperCloudProvider.addEventListener('change', () => {
  const p = whisperCloudProvider.value;
  if (p === 'groq') {
    whisperCloudBaseUrlInput.value = 'https://api.groq.com/openai/v1';
    whisperCloudModelInput.value = 'whisper-large-v3-turbo';
  } else if (p === 'openai') {
    whisperCloudBaseUrlInput.value = 'https://api.openai.com/v1';
    whisperCloudModelInput.value = 'whisper-1';
  }
  persistSettings();
});

testWhisperCloudBtn.addEventListener('click', async () => {
  whisperCloudTestMsg.textContent = 'Testing...';
  whisperCloudTestMsg.className = 'muted';
  await persistSettings();
  try {
    const msg = await invoke('test_whisper_cloud');
    whisperCloudTestMsg.textContent = `✓ ${msg}`;
    whisperCloudTestMsg.className = 'tool-ok';
  } catch (err) {
    whisperCloudTestMsg.textContent = String(err);
    whisperCloudTestMsg.className = 'error-text';
  }
});

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

ttsModeEdgeBtn.addEventListener('click', () => {
  renderTtsMode('edge');
  persistSettings();
});

ttsModeLocalBtn.addEventListener('click', () => {
  renderTtsMode('vieneu');
  persistSettings();
});

vieneuVoiceSelect.addEventListener('change', () => {
  if (vieneuVoiceSelect.value === '__add_custom__') {
    vieneuCustomNameInput.value = '';
    vieneuRefAudioInput.value = '';
    renderVieneuUI();
    vieneuCustomNameInput.focus();
    return;
  }
  lastValidVieneuVoice = vieneuVoiceSelect.value;
  renderVieneuUI();
  persistSettings();
});

if (vieneuSaveCustomBtn) {
  vieneuSaveCustomBtn.addEventListener('click', async () => {
    vieneuMsg.classList.add('hidden');
    const name = (vieneuCustomNameInput.value || '').trim();
    const refAudio = (vieneuRefAudioInput.value || '').trim();
    if (!name) {
      vieneuMsg.textContent = 'Vui lòng nhập tên cho Custom Voice.';
      vieneuMsg.classList.remove('hidden');
      return;
    }
    if (!refAudio) {
      vieneuMsg.textContent = 'Vui lòng kéo thả hoặc chọn file audio mẫu (3–8s).';
      vieneuMsg.classList.remove('hidden');
      return;
    }
    const id = `${Date.now().toString(36)}`;
    customVoices.push({ id, name, refAudio });
    renderCustomVoicesOptions();
    lastValidVieneuVoice = `custom:${id}`;
    vieneuVoiceSelect.value = lastValidVieneuVoice;
    renderVieneuUI();
    await persistSettings();
  });
}

if (vieneuCancelCustomBtn) {
  vieneuCancelCustomBtn.addEventListener('click', () => {
    vieneuMsg.classList.add('hidden');
    vieneuVoiceSelect.value = lastValidVieneuVoice;
    renderVieneuUI();
  });
}

if (vieneuDelVoiceBtn) {
  vieneuDelVoiceBtn.addEventListener('click', async () => {
    const val = vieneuVoiceSelect.value;
    if (!val.startsWith('custom:')) return;
    const cid = val.slice('custom:'.length);
    customVoices = customVoices.filter((v) => v.id !== cid);
    renderCustomVoicesOptions();
    lastValidVieneuVoice = 'Ngọc Huyền';
    vieneuVoiceSelect.value = 'Ngọc Huyền';
    renderVieneuUI();
    await persistSettings();
  });
}

if (vieneuSidecarBadge) {
  vieneuSidecarBadge.addEventListener('click', async () => {
    if (!vieneuSidecarBadge.textContent.includes('Ready')) return;
    try {
      const status = await invoke('stop_vieneu_sidecar_cmd');
      renderVieneuUI(status);
    } catch {}
  });
}

vieneuRefAudioInput.addEventListener('change', persistSettings);

if (vieneuBrowseAudioBtn) {
  vieneuBrowseAudioBtn.addEventListener('click', async () => {
    try {
      const picked = await invoke('pick_audio_file_cmd');
      if (picked) {
        vieneuRefAudioInput.value = picked;
        persistSettings();
      }
    } catch (err) {
      vieneuMsg.textContent = String(err);
      vieneuMsg.classList.remove('hidden');
    }
  });
}

if (window.__TAURI__?.event?.listen) {
  window.__TAURI__.event.listen('tauri://drag-enter', () => {
    if (vieneuVoiceSelect.value === '__add_custom__' && !ttsLocalPanel.classList.contains('hidden')) {
      vieneuRefAudioInput.style.borderColor = '#3b82f6';
    }
  });
  window.__TAURI__.event.listen('tauri://drag-leave', () => {
    vieneuRefAudioInput.style.borderColor = '';
  });
  window.__TAURI__.event.listen('tauri://drag-drop', (event) => {
    vieneuRefAudioInput.style.borderColor = '';
    if (vieneuVoiceSelect.value !== '__add_custom__' || ttsLocalPanel.classList.contains('hidden')) return;
    const paths = event?.payload?.paths;
    if (Array.isArray(paths) && paths.length > 0) {
      const audioPath = paths.find((p) => /\.(wav|mp3|m4a|aac|flac|ogg)$/i.test(p)) || paths[0];
      if (audioPath) {
        vieneuRefAudioInput.value = audioPath;
      }
    }
  });
}

let vieneuPreviewAudio = null;
vieneuTestVoiceBtn.addEventListener('click', async () => {
  if (vieneuPreviewAudio && !vieneuPreviewAudio.paused) {
    vieneuPreviewAudio.pause();
    vieneuPreviewAudio = null;
    vieneuTestVoiceBtn.textContent = '▶ Test';
    return;
  }
  vieneuMsg.classList.add('hidden');
  vieneuTestVoiceBtn.disabled = true;
  vieneuTestVoiceBtn.textContent = 'Loading...';
  try {
    await persistSettings();
    const voice = vieneuVoiceSelect.value || 'Ngọc Huyền';
    const sample = 'Xin chào, đây là giọng đọc thử của VieNeu TTS trên YT Cloud Saver.';
    const url = `http://127.0.0.1:4873/tts?voice=${encodeURIComponent(`vieneu:${voice}`)}&text=${encodeURIComponent(sample)}`;
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const blob = await resp.blob();
    const objUrl = URL.createObjectURL(blob);
    const audio = new Audio(objUrl);
    vieneuPreviewAudio = audio;
    vieneuTestVoiceBtn.textContent = '■ Stop';
    audio.onended = () => {
      URL.revokeObjectURL(objUrl);
      if (vieneuPreviewAudio === audio) vieneuPreviewAudio = null;
      vieneuTestVoiceBtn.textContent = '▶ Test';
    };
    audio.onerror = () => {
      URL.revokeObjectURL(objUrl);
      if (vieneuPreviewAudio === audio) vieneuPreviewAudio = null;
      vieneuTestVoiceBtn.textContent = '▶ Test';
    };
    await audio.play();
  } catch (err) {
    vieneuMsg.textContent = `Test voice failed: ${err}`;
    vieneuMsg.classList.remove('hidden');
    vieneuTestVoiceBtn.textContent = '▶ Test';
  } finally {
    renderVieneuUI();
  }
});

vieneuInstallBtn.addEventListener('click', async () => {
  vieneuMsg.classList.add('hidden');
  vieneuInstallBtn.disabled = true;
  vieneuInstallBtn.textContent = 'Installing...';
  try {
    const status = await invoke('install_vieneu_runtime_cmd');
    renderVieneuUI(status);
  } catch (err) {
    vieneuMsg.textContent = String(err);
    vieneuMsg.classList.remove('hidden');
  } finally {
    vieneuInstallBtn.disabled = false;
    vieneuInstallBtn.textContent = 'Install';
  }
});

vieneuRemoveBtn.addEventListener('click', async () => {
  vieneuMsg.classList.add('hidden');
  vieneuRemoveBtn.disabled = true;
  try {
    const status = await invoke('remove_vieneu_runtime_cmd');
    renderVieneuUI(status);
  } catch (err) {
    vieneuMsg.textContent = String(err);
    vieneuMsg.classList.remove('hidden');
  } finally {
    vieneuRemoveBtn.disabled = false;
  }
});

vieneuVariantList.addEventListener('click', async (e) => {
  const dlId = e.target.getAttribute('data-vn-dl');
  const useId = e.target.getAttribute('data-vn-use');
  const rmId = e.target.getAttribute('data-vn-rm');
  if (useId) {
    activeVieneuModel = useId;
    renderVieneuUI();
    persistSettings();
    return;
  }
  if (dlId) {
    vieneuMsg.classList.add('hidden');
    downloadingVieneuModel = dlId;
    renderVieneuUI();
    try {
      const status = await invoke('download_vieneu_model_cmd', { modelName: dlId });
      activeVieneuModel = dlId;
      downloadingVieneuModel = '';
      renderVieneuUI(status);
      persistSettings();
    } catch (err) {
      downloadingVieneuModel = '';
      renderVieneuUI();
      vieneuMsg.textContent = String(err);
      vieneuMsg.classList.remove('hidden');
    }
    return;
  }
  if (rmId) {
    vieneuMsg.classList.add('hidden');
    try {
      const status = await invoke('remove_vieneu_model_cmd', { modelName: rmId });
      if (activeVieneuModel === rmId && status.downloadedModels?.length) {
        activeVieneuModel = status.downloadedModels[0];
        persistSettings();
      }
      renderVieneuUI(status);
    } catch (err) {
      vieneuMsg.textContent = String(err);
      vieneuMsg.classList.remove('hidden');
    }
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
  const toggleBtn = target.closest('[data-export-toggle], [data-rerun-toggle]');
  if (toggleBtn) {
    e.stopPropagation();
    const current = toggleBtn.closest('.export-dropdown');
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
  const retranslateBtn = target.closest('[data-retranslate-sub]');
  const rewhisperBtn = target.closest('[data-rewhisper-sub]');
  const retranslateSub = retranslateBtn?.getAttribute('data-retranslate-sub');
  const rewhisperSub = rewhisperBtn?.getAttribute('data-rewhisper-sub');
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
  } else if (retranslateSub) {
    jobsList.querySelectorAll('.export-dropdown.open').forEach((el) => el.classList.remove('open'));
    try {
      await invoke('retranslate_subtitle_cache', { fileName: retranslateSub });
      await loadState();
    } catch (err) {
      activityHint.classList.remove('hidden');
      activityHint.textContent = `Re-translate failed: ${err}`;
    }
  } else if (rewhisperSub) {
    jobsList.querySelectorAll('.export-dropdown.open').forEach((el) => el.classList.remove('open'));
    try {
      await invoke('rewhisper_subtitle_cache', { fileName: rewhisperSub });
      await loadState();
    } catch (err) {
      activityHint.classList.remove('hidden');
      activityHint.textContent = `Re-whisper failed: ${err}`;
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

async function runUpdateCheck(silent = true) {
  if (!silent) {
    checkUpdateBtn.disabled = true;
    checkUpdateBtn.textContent = 'Checking...';
  }
  try {
    const info = await invoke('check_app_update');
    if (info && info.version) {
      updateTitle.textContent = `Update available: v${info.version}`;
      updateSub.textContent = (info.body || 'Click Update & Restart to install automatically.').split('\n')[0].slice(0, 80);
      updateBanner.classList.remove('hidden');
    } else if (!silent) {
      checkUpdateBtn.textContent = 'Up to date ✓';
      setTimeout(() => { checkUpdateBtn.textContent = '↻ Updates'; }, 2500);
    }
  } catch (err) {
    if (!silent) {
      updateTitle.textContent = 'Update check failed';
      updateSub.textContent = String(err).slice(0, 90);
      updateBanner.classList.remove('hidden');
    }
  } finally {
    if (!silent) {
      checkUpdateBtn.disabled = false;
      if (checkUpdateBtn.textContent === 'Checking...') {
        checkUpdateBtn.textContent = '↻ Updates';
      }
    }
  }
}

checkUpdateBtn.addEventListener('click', () => runUpdateCheck(false));
installUpdateBtn.addEventListener('click', async () => {
  installUpdateBtn.disabled = true;
  installUpdateBtn.textContent = 'Updating...';
  try {
    await invoke('install_app_update');
  } catch (err) {
    installUpdateBtn.disabled = false;
    installUpdateBtn.textContent = 'Update & Restart';
    updateSub.textContent = `Update failed: ${err}`;
  }
});

setTimeout(() => runUpdateCheck(true), 3500);

loadState();
setInterval(loadState, 2000);
