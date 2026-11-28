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
const coverBtns = document.querySelectorAll('.cover-btn');
const showSignsToggle = document.getElementById('showSignsToggle');
const prevOrig = document.getElementById('prevOrig');
const prevTr = document.getElementById('prevTr');
const dubVolSlider = document.getElementById('dubVolSlider');
const dubVolLabel = document.getElementById('dubVolLabel');
const dubGainSlider = document.getElementById('dubGainSlider');
const dubGainLabel = document.getElementById('dubGainLabel');

const remoteInput = document.getElementById('remoteInput');
const dubVoiceSelect = document.getElementById('dubVoiceSelect');
const dubSpeedSelect = document.getElementById('dubSpeedSelect');
const dubSpeedBadge = document.getElementById('dubSpeedBadge');
const pillDub = document.getElementById('pillDub');
const pillSub = document.getElementById('pillSub');
const panelDub = document.getElementById('panelDub');
const panelSub = document.getElementById('panelSub');
const engEdgeBtn = document.getElementById('engEdgeBtn');
const engVieneuBtn = document.getElementById('engVieneuBtn');
const engBrowserBtn = document.getElementById('engBrowserBtn');
const dubEngineBadge = document.getElementById('dubEngineBadge');
const dubVoiceHint = document.getElementById('dubVoiceHint');
const popupTestVoiceBtn = document.getElementById('popupTestVoiceBtn');
const multiVoiceRow = document.getElementById('multiVoiceRow');
const multiVoiceToggle = document.getElementById('multiVoiceToggle');
const singleVoiceBox = document.getElementById('singleVoiceBox');
const multiVoiceBox = document.getElementById('multiVoiceBox');
const dubFemaleSelect = document.getElementById('dubFemaleSelect');
const dubMaleSelect = document.getElementById('dubMaleSelect');
const castBox = document.getElementById('castBox');
const castListEl = document.getElementById('castList');
const autoMergeSmallToggle = document.getElementById('autoMergeSmallToggle');
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
const popSupportBtn = document.getElementById('popSupportBtn');
const popSupportModal = document.getElementById('popSupportModal');
const closePopSupportBtn = document.getElementById('closePopSupportBtn');

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
  dubVoiceGain: 1.3,
  multiVoice: false,
  dubFemaleVoice: 'vieneu:Ngọc Huyền',
  dubMaleVoice: 'vieneu:Hải Đăng',
  autoMergeSmallClusters: false,
  hardsubCover: 'blur',
  showSigns: true,
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
  const curCover = subStyle.hardsubCover || 'blur';
  coverBtns.forEach((b) => b.classList.toggle('active', b.dataset.cover === curCover));
  if (showSignsToggle) showSignsToggle.checked = subStyle.showSigns !== false;
  const curVoice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
  const eng = curVoice === 'browser' ? 'browser' : curVoice.startsWith('vieneu:') ? 'vieneu' : 'edge';
  if (multiVoiceToggle) multiVoiceToggle.checked = Boolean(subStyle.multiVoice);
  if (autoMergeSmallToggle) autoMergeSmallToggle.checked = Boolean(subStyle.autoMergeSmallClusters);
  renderDubEngine(eng, curVoice);
  renderCastSection();
  dubSpeedSelect.value = String(Number(subStyle.dubSpeed ?? 1));
  if (dubSpeedBadge) dubSpeedBadge.textContent = `${Number(subStyle.dubSpeed ?? 1).toFixed(1)}x`;
  const volPct = Math.round(Number(subStyle.dubVolume ?? 0.15) * 100);
  dubVolSlider.value = String(volPct);
  dubVolLabel.textContent = `${volPct}%`;
  const gainPct = Math.round(Number(subStyle.dubVoiceGain ?? 1.3) * 100);
  if (dubGainSlider) dubGainSlider.value = String(gainPct);
  if (dubGainLabel) dubGainLabel.textContent = `${gainPct}%`;

  const a = subStyle.bgOpacity / 100;
  prevTr.style.fontSize = `${Math.min(20, Math.round(subStyle.fontSize * 0.68))}px`;
  prevTr.style.color = subStyle.textColor;
  prevTr.style.background = hexToRgba(subStyle.bgColor, a);
  prevOrig.style.fontSize = `${Math.max(10, Math.round(Math.min(20, subStyle.fontSize * 0.68) * 0.72))}px`;
  prevOrig.style.background = hexToRgba(subStyle.bgColor, (a * 0.85).toFixed(2));
}

const EDGE_VOICES = [
  { value: 'vi-VN-HoaiMyNeural', label: 'Hoài Mỹ (Nữ · Edge AI)' },
  { value: 'vi-VN-NamMinhNeural', label: 'Nam Minh (Nam · Edge AI)' },
  { value: 'en-US-AriaNeural', label: 'Aria (EN · Edge AI)' },
  { value: 'en-US-GuyNeural', label: 'Guy (EN · Edge AI)' },
];

const VIENEU_PRESET_VOICES = [
  { value: 'vieneu:default', label: 'VieNeu · Mặc định (theo Desktop)' },
  { value: 'vieneu:Ngọc Huyền', label: 'VieNeu · Ngọc Huyền (Nữ Bắc)' },
  { value: 'vieneu:Trúc Ly', label: 'VieNeu · Trúc Ly (Nữ Bắc)' },
  { value: 'vieneu:Thùy Dung', label: 'VieNeu · Thùy Dung (Nữ Nam)' },
  { value: 'vieneu:Ngọc Trân', label: 'VieNeu · Ngọc Trân (Nữ Trung)' },
  { value: 'vieneu:Hải Đăng', label: 'VieNeu · Hải Đăng (Nam Bắc)' },
  { value: 'vieneu:Thiện Minh', label: 'VieNeu · Thiện Minh (Nam Bắc)' },
  { value: 'vieneu:Quang Sơn', label: 'VieNeu · Quang Sơn (Nam Trung)' },
  { value: 'vieneu:Thái Sơn', label: 'VieNeu · Thái Sơn (Nam Nam)' },
];

let customVieneuVoices = [];
let vieneuStatusInfo = { runtimeInstalled: false, downloadedModels: [], sidecarRunning: false };

function renderDubEngine(eng, selectedVoice) {
  engEdgeBtn?.classList.toggle('active', eng === 'edge');
  engVieneuBtn?.classList.toggle('active', eng === 'vieneu');
  engBrowserBtn?.classList.toggle('active', eng === 'browser');

  let list = [];
  if (eng === 'vieneu') {
    list = [
      ...customVieneuVoices.map((v) => ({ value: `vieneu:custom:${v.id}`, label: `🎙 ${v.name}` })),
      ...VIENEU_PRESET_VOICES,
    ];
    if (dubVoiceHint) dubVoiceHint.textContent = 'Local Offline';
    if (dubEngineBadge) {
      const ready = vieneuStatusInfo.runtimeInstalled && (vieneuStatusInfo.downloadedModels?.length || 0) > 0;
      dubEngineBadge.textContent = ready
        ? (vieneuStatusInfo.sidecarRunning ? 'VieNeu · Ready' : 'VieNeu · Installed')
        : 'VieNeu · Not installed';
      dubEngineBadge.style.color = ready ? '#86efac' : '#fbbf24';
    }
  } else if (eng === 'browser') {
    list = [{ value: 'browser', label: 'Browser / macOS System TTS' }];
    if (dubVoiceHint) dubVoiceHint.textContent = 'System Fallback';
    if (dubEngineBadge) {
      dubEngineBadge.textContent = 'System TTS';
      dubEngineBadge.style.color = '#9aa3b2';
    }
  } else {
    list = EDGE_VOICES;
    if (dubVoiceHint) dubVoiceHint.textContent = 'Microsoft Neural';
    if (dubEngineBadge) {
      dubEngineBadge.textContent = 'Edge · Online';
      dubEngineBadge.style.color = '#93c5fd';
    }
  }

  dubVoiceSelect.innerHTML = '';
  for (const item of list) {
    const opt = document.createElement('option');
    opt.value = item.value;
    opt.textContent = item.label;
    dubVoiceSelect.appendChild(opt);
  }
  const match = list.find((i) => i.value === selectedVoice);
  dubVoiceSelect.value = match ? selectedVoice : list[0]?.value || 'vi-VN-HoaiMyNeural';

  const isMulti = Boolean(subStyle.multiVoice) && eng !== 'browser';
  multiVoiceRow?.classList.toggle('hidden', eng === 'browser');
  singleVoiceBox?.classList.toggle('hidden', isMulti);
  multiVoiceBox?.classList.toggle('hidden', !isMulti);

  if (dubFemaleSelect && dubMaleSelect) {
    const fOpts = eng === 'edge'
      ? [{ value: 'vi-VN-HoaiMyNeural', label: 'Hoài Mỹ (Edge)' }]
      : [
          ...customVieneuVoices.map((v) => ({ value: `vieneu:custom:${v.id}`, label: `🎙 ${v.name}` })),
          ...VIENEU_PRESET_VOICES.filter((v) => v.value !== 'vieneu:default'),
        ];
    const mOpts = eng === 'edge'
      ? [{ value: 'vi-VN-NamMinhNeural', label: 'Nam Minh (Edge)' }]
      : [
          ...customVieneuVoices.map((v) => ({ value: `vieneu:custom:${v.id}`, label: `🎙 ${v.name}` })),
          ...VIENEU_PRESET_VOICES.filter((v) => v.value !== 'vieneu:default'),
        ];
    const shortLabel = (s) => s.replace(/^VieNeu · /, '');
    dubFemaleSelect.innerHTML = fOpts.map((o) => `<option value="${o.value}">${shortLabel(o.label)}</option>`).join('');
    dubMaleSelect.innerHTML = mOpts.map((o) => `<option value="${o.value}">${shortLabel(o.label)}</option>`).join('');
    dubFemaleSelect.value = fOpts.some((o) => o.value === subStyle.dubFemaleVoice)
      ? subStyle.dubFemaleVoice
      : fOpts[0]?.value || '';
    dubMaleSelect.value = mOpts.some((o) => o.value === subStyle.dubMaleVoice)
      ? subStyle.dubMaleVoice
      : (mOpts.find((o) => o.value.includes('Hải Đăng'))?.value || mOpts[0]?.value || '');
  }
}

async function saveSubStyle() {
  renderSubStyle();
  await chrome.storage.local.set({ subStyle });
}

async function loadCustomVieneuVoices() {
  try {
    const resp = await fetch('http://127.0.0.1:4873/vieneu/status');
    if (!resp.ok) return;
    const data = await resp.json();
    vieneuStatusInfo = data;
    customVieneuVoices = Array.isArray(data.customVoices) ? data.customVoices : [];
    renderSubStyle();
  } catch {
    // Desktop offline
  }
}

let activeEpisodeId = '';
let activeEpisodeBundle = null;

function getSpeakerVoicePool(eng) {
  if (eng === 'edge') return EDGE_VOICES;
  return [
    ...customVieneuVoices.map((v) => ({ value: `vieneu:custom:${v.id}`, label: `🎙 ${v.name}` })),
    ...VIENEU_PRESET_VOICES.filter((v) => v.value !== 'vieneu:default'),
  ];
}

function getDefaultVoiceForCode(code, idx, eng) {
  const sp = String(code || '');
  if (eng === 'edge') {
    return sp.startsWith('M:') ? 'vi-VN-NamMinhNeural' : 'vi-VN-HoaiMyNeural';
  }
  if (sp.startsWith('M:')) {
    const mPool = [
      subStyle.dubMaleVoice || 'vieneu:Hải Đăng',
      'vieneu:Thiện Minh',
      'vieneu:Quang Sơn',
      'vieneu:Thái Sơn',
    ].filter((v, i, a) => v && a.indexOf(v) === i);
    return mPool[idx % mPool.length];
  }
  const fPool = [
    subStyle.dubFemaleVoice || 'vieneu:Ngọc Huyền',
    'vieneu:Thùy Dung',
    'vieneu:Trúc Ly',
    'vieneu:Ngọc Trân',
  ].filter((v, i, a) => v && a.indexOf(v) === i);
  return fPool[idx % fPool.length];
}

function renderCastSection() {
  if (!castBox || !castListEl) return;
  const curVoice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
  const eng = curVoice === 'browser' ? 'browser' : curVoice.startsWith('vieneu:') ? 'vieneu' : 'edge';
  const cues = activeEpisodeBundle?.cues;
  if (!subStyle.multiVoice || eng === 'browser' || !Array.isArray(cues) || !cues.length) {
    castBox.classList.add('hidden');
    return;
  }

  const counts = new Map();
  for (const c of cues) {
    if (c?.speaker) {
      counts.set(c.speaker, (counts.get(c.speaker) || 0) + 1);
    }
  }
  if (!counts.size) {
    castBox.classList.add('hidden');
    return;
  }

  castBox.classList.remove('hidden');
  const sorted = [...counts.entries()].sort((a, b) => b[1] - a[1]);
  const leadMale = sorted.find(([c]) => c.startsWith('M:'))?.[0] || '';
  const leadFemale = sorted.find(([c]) => c.startsWith('F:'))?.[0] || '';
  const castMap = activeEpisodeBundle.cast || {};
  const overrides = activeEpisodeBundle.speakerOverrides || {};
  const pool = getSpeakerVoicePool(eng);

  let mSeen = 0;
  let fSeen = 0;
  castListEl.innerHTML = sorted
    .map(([code, cnt]) => {
      const isMale = code.startsWith('M:');
      const slotIdx = isMale ? mSeen++ : fSeen++;
      const isLead = code === leadMale || code === leadFemale;
      const ov = overrides[code] || '';
      const isMerged = ov.startsWith('merge:');
      const mergeTarget = isMerged ? ov.slice('merge:'.length) : '';
      const defName = isMale
        ? code === leadMale
          ? 'Nam chính'
          : `Nam phụ ${code.split(':')[1] || ''}`
        : code === leadFemale
          ? 'Nữ chính'
          : `Nữ phụ ${code.split(':')[1] || ''}`;
      const displayName = castMap[code] || defName;
      const selectedVoice = !isMerged && ov ? ov : getDefaultVoiceForCode(code, slotIdx, eng);
      const mergeTargetName = mergeTarget ? (castMap[mergeTarget] || mergeTarget) : '';
      const badgeCol = isMale ? '#60a5fa' : '#f472b6';

      const voiceCtrl = isMerged
        ? `<span style="font-size:10.5px;color:#86efac;flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">🔗 Chung giọng: ${escapeHtml(mergeTargetName)}</span>`
        : `<select data-cast-voice="${escapeHtml(code)}" class="select-full" style="padding:3px 6px;font-size:11px;min-width:0;flex:1;">
            ${pool.map((v) => `<option value="${escapeHtml(v.value)}" ${v.value === selectedVoice ? 'selected' : ''}>${escapeHtml(v.label.replace(/^VieNeu · /, ''))}</option>`).join('')}
          </select>`;

      const mergeBtn = !isLead
        ? `<button type="button" data-cast-merge="${escapeHtml(code)}" data-cast-target="${escapeHtml(isMale ? (leadMale || leadFemale) : (leadFemale || leadMale))}" class="btn-xs-inline" style="padding:3px 6px;font-size:10.5px;${isMerged ? 'border-color:#3b82f6;color:#93c5fd;' : ''}" title="Gộp vào giọng nhân vật chính">${isMerged ? 'Tách' : '🔗 Gộp'}</button>`
        : '';

      return `
        <div class="mini-box" style="padding:6px 8px;gap:4px;display:flex;flex-direction:column;">
          <div class="between" style="gap:6px;">
            <div style="display:flex;align-items:center;gap:5px;min-width:0;flex:1;">
              <span style="font-family:monospace;font-size:10.5px;font-weight:700;color:${badgeCol};background:rgba(255,255,255,0.06);padding:1px 5px;border-radius:4px;flex-shrink:0;">${escapeHtml(code)}</span>
              <input type="text" data-cast-name="${escapeHtml(code)}" value="${escapeHtml(displayName)}" style="padding:2px 5px;font-size:11px;background:transparent;border:1px solid transparent;border-radius:4px;color:#e6e8ec;min-width:0;flex:1;" title="Bấm để sửa tên nhân vật" />
            </div>
            <span class="label-sm" style="font-size:10px;flex-shrink:0;">${cnt} câu</span>
          </div>
          <div style="display:flex;align-items:center;gap:4px;">
            ${voiceCtrl}
            <button type="button" data-cast-swap="${escapeHtml(code)}" class="btn-xs-inline" style="padding:3px 6px;font-size:10.5px;" title="Đảo nhanh giọng Nam ↔ Nữ">⇄</button>
            ${mergeBtn}
          </div>
        </div>
      `;
    })
    .join('');
}

async function persistActiveCastUpdate(nextCast, nextOverrides) {
  if (!activeEpisodeId) return;
  try {
    const updated = await sendBg('UPDATE_SUBTITLE_CAST', {
      videoId: activeEpisodeId,
      cast: nextCast,
      speakerOverrides: nextOverrides,
    });
    activeEpisodeBundle = updated;
    renderCastSection();
    await chrome.storage.local.set({ castSyncTick: Date.now() });
  } catch {
    // Ignore if offline
  }
}

async function loadActiveEpisodeCast() {
  try {
    const active = await sendBg('GET_ACTIVE_VIDEO');
    const vid = active?.videoId || '';
    if (!vid) return;
    if (vid !== activeEpisodeId || !activeEpisodeBundle) {
      activeEpisodeId = vid;
      const bundle = await sendBg('GET_SUBTITLES', { videoId: vid }).catch(() => null);
      if (bundle) {
        activeEpisodeBundle = bundle;
        renderCastSection();
      }
    }
  } catch {
    // Ignore
  }
}

async function loadSubStyle() {
  const res = await chrome.storage.local.get({ subStyle: DEFAULT_SUB_STYLE });
  subStyle = { ...DEFAULT_SUB_STYLE, ...(res.subStyle || {}) };
  renderSubStyle();
  loadCustomVieneuVoices();
  loadActiveEpisodeCast();
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
          ${
            job.sizeText || job.speedText
              ? `<div class="job-stats"><span class="size-val">${escapeHtml(job.sizeText || '')}</span><span>${escapeHtml(job.speedText || '')}</span></div>`
              : ''
          }
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

function switchSubPill(which) {
  const isDub = which === 'dub';
  pillDub?.classList.toggle('active', isDub);
  pillSub?.classList.toggle('active', !isDub);
  panelDub?.classList.toggle('hidden', !isDub);
  panelSub?.classList.toggle('hidden', isDub);
}

pillDub?.addEventListener('click', () => switchSubPill('dub'));
pillSub?.addEventListener('click', () => switchSubPill('sub'));

engEdgeBtn?.addEventListener('click', () => {
  subStyle.dubVoice = 'vi-VN-HoaiMyNeural';
  saveSubStyle();
});
engVieneuBtn?.addEventListener('click', () => {
  const firstCustom = customVieneuVoices[0]?.id ? `vieneu:custom:${customVieneuVoices[0].id}` : 'vieneu:default';
  subStyle.dubVoice = firstCustom;
  saveSubStyle();
});
engBrowserBtn?.addEventListener('click', () => {
  subStyle.dubVoice = 'browser';
  saveSubStyle();
});

let popupPreviewAudio = null;
popupTestVoiceBtn?.addEventListener('click', async () => {
  if (popupPreviewAudio && !popupPreviewAudio.paused) {
    popupPreviewAudio.pause();
    popupPreviewAudio = null;
    popupTestVoiceBtn.textContent = '▶ Test';
    return;
  }
  const voice = dubVoiceSelect.value || 'vi-VN-HoaiMyNeural';
  const sample = voice.startsWith('en-')
    ? 'Hello, this is a voice preview from YT Cloud Saver.'
    : 'Xin chào, đây là giọng đọc thử của YT Cloud Saver.';
  if (voice === 'browser') {
    if (window.speechSynthesis) {
      window.speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(sample);
      u.lang = 'vi-VN';
      window.speechSynthesis.speak(u);
    }
    return;
  }
  popupTestVoiceBtn.disabled = true;
  popupTestVoiceBtn.textContent = 'Loading...';
  try {
    const url = `http://127.0.0.1:4873/tts?voice=${encodeURIComponent(voice)}&text=${encodeURIComponent(sample)}`;
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const blob = await resp.blob();
    const objUrl = URL.createObjectURL(blob);
    const audio = new Audio(objUrl);
    popupPreviewAudio = audio;
    popupTestVoiceBtn.textContent = '■ Stop';
    audio.onended = () => {
      URL.revokeObjectURL(objUrl);
      if (popupPreviewAudio === audio) popupPreviewAudio = null;
      popupTestVoiceBtn.textContent = '▶ Test';
    };
    await audio.play();
  } catch {
    popupTestVoiceBtn.textContent = '▶ Test';
  } finally {
    popupTestVoiceBtn.disabled = false;
  }
});

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
coverBtns.forEach((b) =>
  b.addEventListener('click', () => {
    subStyle.hardsubCover = b.dataset.cover || 'blur';
    saveSubStyle();
  }),
);
showSignsToggle?.addEventListener('change', () => {
  subStyle.showSigns = showSignsToggle.checked;
  saveSubStyle();
});
dubVoiceSelect.addEventListener('change', () => {
  subStyle.dubVoice = dubVoiceSelect.value;
  saveSubStyle();
});
multiVoiceToggle?.addEventListener('change', () => {
  subStyle.multiVoice = multiVoiceToggle.checked;
  saveSubStyle();
});
autoMergeSmallToggle?.addEventListener('change', () => {
  subStyle.autoMergeSmallClusters = autoMergeSmallToggle.checked;
  saveSubStyle();
  chrome.storage.local.set({ castSyncTick: Date.now() });
});
castListEl?.addEventListener('change', (e) => {
  const voiceCode = e.target.getAttribute('data-cast-voice');
  const nameCode = e.target.getAttribute('data-cast-name');
  if (voiceCode && activeEpisodeBundle) {
    const nextOv = { ...(activeEpisodeBundle.speakerOverrides || {}), [voiceCode]: e.target.value };
    persistActiveCastUpdate(null, nextOv);
  } else if (nameCode && activeEpisodeBundle) {
    const nextCast = { ...(activeEpisodeBundle.cast || {}), [nameCode]: e.target.value.trim() };
    persistActiveCastUpdate(nextCast, null);
  }
});
castListEl?.addEventListener('click', (e) => {
  const mergeBtn = e.target.closest('[data-cast-merge]');
  const swapBtn = e.target.closest('[data-cast-swap]');
  if (mergeBtn && activeEpisodeBundle) {
    const code = mergeBtn.getAttribute('data-cast-merge');
    const target = mergeBtn.getAttribute('data-cast-target');
    const nextOv = { ...(activeEpisodeBundle.speakerOverrides || {}) };
    if (String(nextOv[code] || '').startsWith('merge:')) {
      delete nextOv[code];
    } else if (target) {
      nextOv[code] = `merge:${target}`;
    }
    persistActiveCastUpdate(null, nextOv);
  } else if (swapBtn && activeEpisodeBundle) {
    const code = swapBtn.getAttribute('data-cast-swap');
    const curVoice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
    const eng = curVoice.startsWith('vieneu:') ? 'vieneu' : 'edge';
    const nextOv = { ...(activeEpisodeBundle.speakerOverrides || {}) };
    const curAssigned = nextOv[code] || '';
    const isCurrentlyMale =
      curAssigned.includes('NamMinh') ||
      curAssigned.includes('Hải Đăng') ||
      curAssigned.includes('Thiện Minh') ||
      curAssigned.includes('Quang Sơn') ||
      curAssigned.includes('Thái Sơn') ||
      (!curAssigned && code.startsWith('M:'));
    nextOv[code] = isCurrentlyMale
      ? eng === 'edge'
        ? 'vi-VN-HoaiMyNeural'
        : subStyle.dubFemaleVoice || 'vieneu:Ngọc Huyền'
      : eng === 'edge'
        ? 'vi-VN-NamMinhNeural'
        : subStyle.dubMaleVoice || 'vieneu:Hải Đăng';
    persistActiveCastUpdate(null, nextOv);
  }
});
dubFemaleSelect?.addEventListener('change', () => {
  subStyle.dubFemaleVoice = dubFemaleSelect.value;
  saveSubStyle();
});
dubMaleSelect?.addEventListener('change', () => {
  subStyle.dubMaleVoice = dubMaleSelect.value;
  saveSubStyle();
});
dubSpeedSelect.addEventListener('change', () => {
  subStyle.dubSpeed = Number(dubSpeedSelect.value);
  saveSubStyle();
});
dubVolSlider.addEventListener('input', () => {
  subStyle.dubVolume = Number(dubVolSlider.value) / 100;
  saveSubStyle();
});
dubGainSlider?.addEventListener('input', () => {
  subStyle.dubVoiceGain = Number(dubGainSlider.value) / 100;
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
popSupportBtn.addEventListener('click', () => popSupportModal.classList.remove('hidden'));
closePopSupportBtn.addEventListener('click', () => popSupportModal.classList.add('hidden'));
popSupportModal.addEventListener('click', (e) => {
  if (e.target === popSupportModal) popSupportModal.classList.add('hidden');
});
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
