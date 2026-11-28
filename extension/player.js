(async () => {
  const params = new URLSearchParams(location.search);
  const videoId = params.get('videoId');
  const playerWrap = document.getElementById('playerWrap');
  const videoEl = document.getElementById('video');
  const badgeEl = document.getElementById('badge');
  const errorBox = document.getElementById('errorBox');
  const errorSub = document.getElementById('errorSub');
  const subBtn = document.getElementById('subBtn');
  const dubBtn = document.getElementById('dubBtn');
  const fsBtn = document.getElementById('fsBtn');
  const closeBtn = document.getElementById('closeBtn');
  const subOverlay = document.getElementById('subOverlay');
  const playBtn = document.getElementById('playBtn');
  const muteBtn = document.getElementById('muteBtn');
  const volRange = document.getElementById('volRange');
  const timeText = document.getElementById('timeText');
  const seekWrap = document.getElementById('seekWrap');
  const seekFill = document.getElementById('seekFill');
  const seekBuf = document.getElementById('seekBuf');
  const speedBtn = document.getElementById('speedBtn');

  if (!videoId) {
    errorBox.classList.add('visible');
    errorSub.textContent = 'Missing videoId.';
    return;
  }

  function sendBg(type, payload = {}) {
    return new Promise((resolve, reject) => {
      chrome.runtime.sendMessage({ type, ...payload }, (res) => {
        if (chrome.runtime.lastError) return reject(new Error(chrome.runtime.lastError.message));
        if (!res?.ok) return reject(new Error(res?.error || 'Request failed'));
        resolve(res.data);
      });
    });
  }

  let subBundle = null;
  let subMode = 'off';
  let dubEnabled = false;
  let lastSpokenCueIdx = -1;
  let savedVolume = null;
  let syncRaf = null;
  let speakSeq = 0;
  let speakTimer = null;
  let dubAudioEl = null;
  const ttsAudioCache = new Map();
  let preparedUpToIdx = -1;
  let prepareBatchInFlight = false;
  let prefetchInFlight = false;

  let subStyle = {
    defaultMode: 'bilingual',
    fontSize: 18,
    textColor: '#ffffff',
    bgColor: '#000000',
    bgOpacity: 80,
    dubVoice: 'vi-VN-HoaiMyNeural',
    dubSpeed: 1.0,
    dubVolume: 0.15,
    multiVoice: false,
    dubFemaleVoice: 'vieneu:Ngọc Huyền',
    dubMaleVoice: 'vieneu:Hải Đăng',
    autoMergeSmallClusters: false,
    hardsubCover: 'blur',
    showSigns: true,
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
        subCycleStartMode = subStyle.defaultMode;
        updateButtons();
        if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
      }
      if (dubEnabled && savedVolume !== null) {
        videoEl.volume = Math.min(savedVolume, Number(subStyle.dubVolume ?? 0.15));
      }
    }
    if (area === 'local' && changes.castSyncTick?.newValue) {
      sendBg('GET_SUBTITLES', { videoId })
        .then((fresh) => {
          if (fresh) {
            subBundle = fresh;
            ttsAudioCache.clear();
            preparedUpToIdx = -1;
          }
        })
        .catch(() => {});
    }
  });

  function hexToRgba(hex, alpha) {
    const clean = (hex || '#000000').replace('#', '');
    const r = parseInt(clean.slice(0, 2), 16) || 0;
    const g = parseInt(clean.slice(2, 4), 16) || 0;
    const b = parseInt(clean.slice(4, 6), 16) || 0;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  let translateInFlight = null;
  let activeTranslatePct = null;

  function updateButtons() {
    const pctSuffix = activeTranslatePct !== null ? ` (${activeTranslatePct}%)` : '';
    subBtn.dataset.state =
      subMode === 'bilingual'
        ? 'active'
        : subMode === 'translated'
          ? 'translated'
          : activeTranslatePct !== null
            ? 'active'
            : 'idle';
    const baseLabel =
      subMode === 'bilingual'
        ? 'AI CC: Bilingual'
        : subMode === 'translated'
          ? 'AI CC: Translated'
          : 'AI CC: OFF';
    subBtn.querySelector('.txt').textContent = baseLabel + pctSuffix;

    dubBtn.dataset.state = dubEnabled ? 'done' : 'idle';
    dubBtn.querySelector('.txt').textContent = dubEnabled ? 'Dub: ON' : 'Dub: OFF';
  }

  function restoreVideoVolume() {
    if (savedVolume !== null) {
      videoEl.volume = savedVolume;
      savedVolume = null;
    }
  }

  function pauseDubbingSpeech() {
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
    restoreVideoVolume();
  }

  function resumeDubbingIfSameCue(curIdx) {
    if (!dubEnabled || videoEl.paused) return false;
    if (curIdx >= 0 && curIdx === lastSpokenCueIdx) {
      if (dubAudioEl && dubAudioEl.src && dubAudioEl.paused && !dubAudioEl.ended && dubAudioEl.currentTime > 0) {
        duckVideoVolume();
        dubAudioEl.play().catch(() => {});
        return true;
      }
      if (window.speechSynthesis?.paused) {
        duckVideoVolume();
        window.speechSynthesis.resume();
        return true;
      }
    }
    return false;
  }

  function stopDubbingSpeech(resetCue = false) {
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
    restoreVideoVolume();
  }

  videoEl.addEventListener('pause', () => {
    if (dubEnabled) pauseDubbingSpeech();
  });
  videoEl.addEventListener('play', () => {
    if (!dubEnabled || !subBundle?.cues?.length) return;
    const nowMs = videoEl.currentTime * 1000;
    const curIdx = subBundle.cues.findIndex((c) => nowMs >= c.startMs && nowMs <= c.endMs);
    resumeDubbingIfSameCue(curIdx);
  });
  videoEl.addEventListener('seeking', () => {
    if (dubEnabled) stopDubbingSpeech(true);
  });
  videoEl.addEventListener('ended', () => {
    if (dubEnabled) stopDubbingSpeech(true);
  });

  function computeDubRate(text, cue) {
    const playbackRate = Math.max(0.5, Number(videoEl.playbackRate) || 1);
    const cueMs = Math.max(400, (cue.endMs || 0) - (cue.startMs || 0));
    const availableSec = cueMs / 1000 / playbackRate;
    const clean = text.trim();
    const words = clean ? clean.split(/\s+/).length : 1;
    const estNaturalSec = Math.max(words / 2.7, clean.length / 14);
    const fitRate = estNaturalSec / Math.max(0.35, availableSec * 0.92);
    const userSpeed = Math.max(0.7, Math.min(2.0, Number(subStyle.dubSpeed) || 1.0));
    return Math.min(3.5, Math.max(userSpeed * playbackRate, fitRate));
  }

  function fetchEdgeTtsAudio(text, voice, rateStr) {
    const key = `${voice}|${rateStr}|${text}`;
    if (ttsAudioCache.has(key)) return ttsAudioCache.get(key);
    const runFetch = async () => {
      try {
        const res = await sendBg('SYNTH_TTS', { text, voice, rate: rateStr });
        if (!res?.audioUrl) throw new Error('Empty Edge TTS audio');
        return res.audioUrl;
      } catch (firstErr) {
        await new Promise((r) => setTimeout(r, 150));
        const res2 = await sendBg('SYNTH_TTS', { text, voice, rate: rateStr });
        if (!res2?.audioUrl) throw firstErr;
        return res2.audioUrl;
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
      if (!keepKeys.has(key)) ttsAudioCache.delete(key);
    }
  }

  async function prepareBatchAhead(startIdx, count = 20) {
    const voice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
    if (voice === 'browser' || !subBundle?.cues?.length) return;
    if (prepareBatchInFlight || startIdx + 6 < preparedUpToIdx) return;

    const from = Math.max(0, startIdx);
    const to = Math.min(subBundle.cues.length, from + count);
    const texts = [];
    for (let i = from; i < to; i += 1) {
      const t = subBundle.cues[i]?.translated?.trim();
      if (t) texts.push(t);
    }
    if (!texts.length) return;

    prepareBatchInFlight = true;
    try {
      await sendBg('PREPARE_TTS', { texts, voice, rate: '+15%' }).catch(() => {});
      preparedUpToIdx = Math.max(preparedUpToIdx, to);
    } finally {
      prepareBatchInFlight = false;
    }
  }

  async function prefetchNextCues(idx) {
    const voice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
    if (voice === 'browser' || !subBundle?.cues?.length || prefetchInFlight) return;
    pruneRamCacheWindow(idx, voice, '+15%');
    prepareBatchAhead(idx + 4, 20);
    prefetchInFlight = true;
    try {
      for (let offset = 1; offset <= 5; offset += 1) {
        if (!dubEnabled) break;
        const nextText = subBundle.cues[idx + offset]?.translated?.trim();
        if (nextText) {
          await fetchEdgeTtsAudio(nextText, voice, '+15%').catch(() => {});
        }
      }
    } finally {
      prefetchInFlight = false;
    }
  }

  function duckVideoVolume() {
    if (savedVolume === null) savedVolume = videoEl.volume;
    videoEl.volume = Math.min(savedVolume, Number(subStyle.dubVolume ?? 0.15));
  }

  function speakCueBrowser(text, cue, mySeq) {
    if (!window.speechSynthesis) return;
    if (window.speechSynthesis.speaking || window.speechSynthesis.pending) {
      window.speechSynthesis.cancel();
    }
    const utter = new SpeechSynthesisUtterance(text);
    utter.lang = 'vi-VN';
    utter.rate = Math.min(2.0, computeDubRate(text, cue));
    const finish = () => {
      if (mySeq === speakSeq) restoreVideoVolume();
    };
    utter.onend = finish;
    utter.onerror = finish;
    speakTimer = setTimeout(() => {
      speakTimer = null;
      if (mySeq !== speakSeq || !dubEnabled || videoEl.paused) return;
      duckVideoVolume();
      window.speechSynthesis.speak(utter);
    }, 25);
  }

  async function speakCue(cue, idx) {
    const text = cue?.translated?.trim();
    if (!text) return;

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

    let voice = subStyle.dubVoice || 'vi-VN-HoaiMyNeural';
    if (voice !== 'browser' && subStyle.multiVoice && cue?.speaker) {
      let sp = String(cue.speaker);
      const ov = subBundle?.speakerOverrides?.[sp];
      if (ov && ov.startsWith('merge:')) sp = ov.slice('merge:'.length);
      const direct = subBundle?.speakerOverrides?.[sp];
      if (direct && !direct.startsWith('merge:')) {
        voice = direct;
      } else if (!voice.startsWith('vieneu:')) {
        if (sp.startsWith('F:')) voice = 'vi-VN-HoaiMyNeural';
        else if (sp.startsWith('M:')) voice = 'vi-VN-NamMinhNeural';
      } else {
        voice = sp.startsWith('M:')
          ? subStyle.dubMaleVoice || 'vieneu:Hải Đăng'
          : subStyle.dubFemaleVoice || 'vieneu:Ngọc Huyền';
      }
    }
    if (voice === 'browser') {
      speakCueBrowser(text, cue, mySeq);
      return;
    }

    prefetchNextCues(idx);
    try {
      const audioUrl = await fetchEdgeTtsAudio(text, voice, '+15%');
      if (mySeq !== speakSeq || !dubEnabled || videoEl.paused) {
        if (!dubAudioEl || dubAudioEl.paused) restoreVideoVolume();
        return;
      }
      if (!dubAudioEl) dubAudioEl = new Audio();
      dubAudioEl.onended = () => {
        if (mySeq === speakSeq) restoreVideoVolume();
      };
      dubAudioEl.onerror = () => {
        if (mySeq === speakSeq) restoreVideoVolume();
      };
      dubAudioEl.src = audioUrl;
      await new Promise((resolve) => {
        if (Number.isFinite(dubAudioEl.duration) && dubAudioEl.duration > 0) {
          resolve();
          return;
        }
        const onMeta = () => {
          dubAudioEl.removeEventListener('loadedmetadata', onMeta);
          dubAudioEl.removeEventListener('error', onMeta);
          resolve();
        };
        dubAudioEl.addEventListener('loadedmetadata', onMeta, { once: true });
        dubAudioEl.addEventListener('error', onMeta, { once: true });
        setTimeout(onMeta, 120);
      });
      if (mySeq !== speakSeq || !dubEnabled || videoEl.paused) {
        if (!dubAudioEl || dubAudioEl.paused) restoreVideoVolume();
        return;
      }

      const vidRate = Math.max(0.5, Number(videoEl.playbackRate) || 1);
      const userSpeed = Math.max(0.7, Math.min(2.0, Number(subStyle.dubSpeed) || 1.0));
      const baseAudioRate = (userSpeed * vidRate) / 1.15;
      let exactAudioRate = baseAudioRate;
      if (Number.isFinite(dubAudioEl.duration) && dubAudioEl.duration > 0) {
        const remCueMs = Math.max(300, (cue.endMs || 0) - videoEl.currentTime * 1000);
        const availSec = remCueMs / 1000 / vidRate;
        const fitAudioRate = dubAudioEl.duration / Math.max(0.25, availSec * 0.96);
        exactAudioRate = Math.max(baseAudioRate, fitAudioRate);
      } else {
        exactAudioRate = computeDubRate(text, cue) / 1.15;
      }
      dubAudioEl.playbackRate = Math.min(2.5, Math.max(0.7, exactAudioRate));
      duckVideoVolume();
      await dubAudioEl.play();
    } catch {
      if (mySeq !== speakSeq || !dubEnabled || videoEl.paused) {
        restoreVideoVolume();
        return;
      }
      speakCueBrowser(text, cue, mySeq);
    }
  }

  let lastSignRenderKey = '';

  function escapeHtmlSafe(str) {
    return String(str || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function renderCloudPlayerSigns(nowMs) {
    let signLayer = document.getElementById('cloudSignOverlay');
    if (subMode === 'off' || subStyle.showSigns === false || !subBundle?.signs?.length) {
      if (signLayer) signLayer.innerHTML = '';
      lastSignRenderKey = '';
      return;
    }
    if (!signLayer) {
      signLayer = document.createElement('div');
      signLayer.id = 'cloudSignOverlay';
      Object.assign(signLayer.style, {
        position: 'absolute',
        pointerEvents: 'none',
        zIndex: '18',
        overflow: 'hidden',
      });
      playerWrap.appendChild(signLayer);
    }
    const pr = playerWrap.getBoundingClientRect();
    const vw = videoEl.videoWidth || pr.width || 640;
    const vh = videoEl.videoHeight || pr.height || 360;
    const vidAspect = vw / vh;
    const boxAspect = (pr.width || 640) / (pr.height || 360);
    let rw = pr.width;
    let rh = pr.height;
    if (boxAspect > vidAspect + 0.01) rw = pr.height * vidAspect;
    else if (boxAspect < vidAspect - 0.01) rh = pr.width / vidAspect;
    const left = (pr.width - rw) * 0.5;
    const top = (pr.height - rh) * 0.5;
    signLayer.style.left = `${Math.round(left)}px`;
    signLayer.style.top = `${Math.round(top)}px`;
    signLayer.style.width = `${Math.round(rw)}px`;
    signLayer.style.height = `${Math.round(rh)}px`;

    const active = subBundle.signs.filter(
      (s) => s.translated && nowMs >= s.startMs && nowMs <= s.endMs,
    );
    const key = `${Math.round(rh)}|${active.map((s) => `${s.startMs}:${s.translated}`).join('||')}`;
    if (key === lastSignRenderKey) return;
    lastSignRenderKey = key;
    if (!active.length) {
      signLayer.innerHTML = '';
      return;
    }
    signLayer.innerHTML = active
      .map((s) => {
        const fg = s.fgHex || '#FFFFFF';
        const bg = hexToRgba(s.bgHex || '#111827', s.solidPlate ? 0.94 : 0.80);
        const fontFam =
          s.fontStyle === 'serif'
            ? "Georgia, 'Noto Serif', serif"
            : "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
        const pxH = Math.max(18, (s.hPct / 100) * rh);
        const fontPx = Math.max(11, Math.min(24, Math.round(pxH * 0.40)));
        return `<div style="position:absolute;left:${Math.max(0.5, s.xPct - 0.5).toFixed(2)}%;top:${Math.max(0.5, s.yPct - 0.4).toFixed(2)}%;min-width:${(s.wPct + 1).toFixed(2)}%;min-height:${(s.hPct + 0.8).toFixed(2)}%;max-width:68%;background:${bg};backdrop-filter:blur(9px);-webkit-backdrop-filter:blur(9px);color:${fg};font-family:${fontFam};padding:4px 10px;border-radius:6px;border:1px solid rgba(255,255,255,0.16);font-weight:700;font-size:${fontPx}px;display:flex;align-items:center;justify-content:center;text-align:center;">${escapeHtmlSafe(s.translated)}</div>`;
      })
      .join('');
  }

  function tickSubtitles() {
    if ((subBundle?.cues?.length || subBundle?.signs?.length) && (subMode !== 'off' || dubEnabled)) {
      const nowMs = videoEl.currentTime * 1000;
      const idx = subBundle.cues ? subBundle.cues.findIndex((c) => nowMs >= c.startMs && nowMs <= c.endMs) : -1;
      const cue = idx >= 0 ? subBundle.cues[idx] : null;

      renderCloudPlayerSigns(nowMs);

      if (subMode !== 'off') {
        if (!cue) {
          subOverlay.innerHTML = '';
        } else {
          const coverMode = subStyle.hardsubCover || 'blur';
          const hasHardsub = Boolean(subBundle?.hardsubBox) && coverMode !== 'off';
          const a = Number(subStyle.bgOpacity ?? 80) / 100;
          const trAlpha = coverMode === 'solid' && hasHardsub ? 0.96 : hasHardsub ? Math.max(a, 0.78) : a;
          const trBg = hexToRgba(subStyle.bgColor, trAlpha.toFixed(2));
          const origBg = hexToRgba(subStyle.bgColor, (trAlpha * 0.86).toFixed(2));
          const blurCss =
            coverMode === 'blur' && hasHardsub
              ? 'backdrop-filter:blur(10px);-webkit-backdrop-filter:blur(10px);min-width:52%;'
              : coverMode === 'solid' && hasHardsub
                ? 'min-width:56%;'
                : '';
          const trSize = Number(subStyle.fontSize || 18);
          const origSize = Math.max(11, Math.round(trSize * 0.78));
          const origHtml =
            subMode === 'bilingual'
              ? `<div style="background:${origBg};${blurCss}color:#e2e8f0;padding:3px 12px;border-radius:6px;font-size:${origSize}px;line-height:1.35;">${escapeHtmlSafe(cue.original)}</div>`
              : '';
          const trHtml = `<div style="background:${trBg};${blurCss}color:${subStyle.textColor || '#ffffff'};padding:6px 16px;border-radius:8px;font-size:${trSize}px;font-weight:600;line-height:1.35;text-shadow:0 1px 3px #000;">${escapeHtmlSafe(cue.translated)}</div>`;
          subOverlay.innerHTML = origHtml + trHtml;
        }
      }

      if (dubEnabled) {
        if (videoEl.paused) {
          if (dubAudioEl && !dubAudioEl.paused) {
            pauseDubbingSpeech();
          }
        } else if (idx >= 0 && idx !== lastSpokenCueIdx) {
          lastSpokenCueIdx = idx;
          speakCue(cue, idx);
        }
      }
    }
    if (subMode !== 'off' || dubEnabled) {
      syncRaf = requestAnimationFrame(tickSubtitles);
    } else {
      syncRaf = null;
    }
  }

  async function ensureSubtitlesLoaded() {
    if (subBundle && (subBundle.cues?.length || subBundle.signs?.length) && !subBundle.partial) {
      return subBundle;
    }
    if (translateInFlight) return translateInFlight;

    translateInFlight = (async () => {
      try {
        const cached = await sendBg('GET_SUBTITLES', { videoId });
        if (cached && (cached.cues?.length || cached.signs?.length) && !cached.partial) {
          subBundle = cached;
          return subBundle;
        }
      } catch {}

      activeTranslatePct = 0;
      updateButtons();
      try {
        const res = await sendBg('TRANSLATE_VIDEO', {
          payload: {
            url: `https://www.youtube.com/watch?v=${videoId}`,
            videoId,
            title: `Cloud Video ${videoId}`,
          },
        });
        if (res?.cached && res?.bundle) {
          subBundle = res.bundle;
          return subBundle;
        }
        if (res?.job) {
          activeTranslatePct = res.job.progress || 5;
          updateButtons();
          while (true) {
            await new Promise((r) => setTimeout(r, 1500));
            const { job } = await sendBg('GET_JOB', { id: res.job.id });
            if (!job) throw new Error('Translation job disappeared');
            if (job.progress >= 15 || job.status === 'done') {
              try {
                const partial = await sendBg('GET_SUBTITLES', { videoId });
                if (partial && (partial.cues?.length || partial.signs?.length)) {
                  subBundle = partial;
                  if ((subMode !== 'off' || dubEnabled) && !syncRaf) {
                    syncRaf = requestAnimationFrame(tickSubtitles);
                  }
                }
              } catch {}
            }
            if (job.status === 'done') {
              subBundle = await sendBg('GET_SUBTITLES', { videoId }).catch(() => subBundle);
              return subBundle;
            }
            if (job.status === 'error' || job.status === 'cancelled') {
              throw new Error(job.error || 'Translation failed');
            }
            activeTranslatePct = job.progress || 0;
            updateButtons();
          }
        }
        throw new Error('No subtitles available');
      } finally {
        translateInFlight = null;
        activeTranslatePct = null;
        updateButtons();
      }
    })();

    return translateInFlight;
  }

  let subCycleStartMode = null;
  subBtn.addEventListener('click', async () => {
    if (subMode === 'off') {
      subCycleStartMode = subStyle.defaultMode || 'bilingual';
      subMode = subCycleStartMode;
      updateButtons();
      if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
      try {
        await ensureSubtitlesLoaded();
        if (subMode !== 'off' && !syncRaf) {
          syncRaf = requestAnimationFrame(tickSubtitles);
        }
      } catch (err) {
        subMode = 'off';
        subCycleStartMode = null;
        subBtn.dataset.state = 'error';
        subBtn.title = err.message;
        subBtn.querySelector('.txt').textContent = 'AI CC: Error';
      }
      return;
    }
    if (subCycleStartMode && subMode === subCycleStartMode) {
      subMode = subCycleStartMode === 'bilingual' ? 'translated' : 'bilingual';
      if (subStyle.defaultMode !== subMode) {
        subStyle = { ...subStyle, defaultMode: subMode };
        chrome.storage.local.set({ subStyle });
      }
      updateButtons();
      if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
      return;
    }
    subMode = 'off';
    subCycleStartMode = null;
    subOverlay.innerHTML = '';
    const cloudSigns = document.getElementById('cloudSignOverlay');
    if (cloudSigns) cloudSigns.innerHTML = '';
    lastSignRenderKey = '';
    updateButtons();
  });

  dubBtn.addEventListener('click', async () => {
    if (!dubEnabled) {
      try {
        await ensureSubtitlesLoaded();
        const nowMs = (videoEl.currentTime || 0) * 1000;
        const curIdx = Math.max(0, subBundle?.cues?.findIndex((c) => c.endMs >= nowMs) ?? 0);
        preparedUpToIdx = -1;
        if ((subStyle.dubVoice || 'vi-VN-HoaiMyNeural') !== 'browser') {
          dubBtn.dataset.state = 'working';
          dubBtn.querySelector('.txt').textContent = 'Dub: Buffering...';
          await prepareBatchAhead(curIdx, 10);
        }
        dubEnabled = true;
        if (subMode === 'off') subMode = subStyle.defaultMode || 'bilingual';
        lastSpokenCueIdx = -1;
        updateButtons();
        if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
      } catch (err) {
        dubBtn.dataset.state = 'error';
        dubBtn.title = err.message;
      }
      return;
    }
    dubEnabled = false;
    stopDubbingSpeech(true);
    updateButtons();
  });

  fsBtn.addEventListener('click', () => {
    if (!document.fullscreenElement) {
      playerWrap.requestFullscreen?.();
    } else {
      document.exitFullscreen?.();
    }
  });

  if (window.self !== window.top && closeBtn) {
    closeBtn.style.display = 'inline-flex';
    closeBtn.addEventListener('click', () => {
      stopDubbingSpeech(true);
      window.parent.postMessage({ type: 'YTCS_CLOSE_CLOUD_PLAYER' }, '*');
    });
  }

  let idleTimer = null;
  function pokeActivity() {
    playerWrap.classList.remove('idle');
    clearTimeout(idleTimer);
    if (!videoEl.paused) {
      idleTimer = setTimeout(() => playerWrap.classList.add('idle'), 2500);
    }
  }
  playerWrap.addEventListener('mousemove', pokeActivity);
  videoEl.addEventListener('pause', pokeActivity);

  function fmtTime(sec) {
    if (!Number.isFinite(sec) || sec < 0) return '0:00';
    const s = Math.floor(sec);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const rem = s % 60;
    const pad = (n) => String(n).padStart(2, '0');
    return h > 0 ? `${h}:${pad(m)}:${pad(rem)}` : `${m}:${pad(rem)}`;
  }

  function syncPlayerUi() {
    playBtn.textContent = videoEl.paused ? '▶' : '⏸';
    muteBtn.textContent = videoEl.muted || videoEl.volume === 0 ? '🔇' : '🔊';
    if (!dubEnabled) {
      volRange.value = String(videoEl.muted ? 0 : videoEl.volume);
    }
    const dur = videoEl.duration || 0;
    const cur = videoEl.currentTime || 0;
    timeText.textContent = `${fmtTime(cur)} / ${fmtTime(dur)}`;
    seekFill.style.width = dur > 0 ? `${(cur / dur) * 100}%` : '0%';
    if (dur > 0 && videoEl.buffered?.length) {
      const bufEnd = videoEl.buffered.end(videoEl.buffered.length - 1);
      seekBuf.style.width = `${Math.min(100, (bufEnd / dur) * 100)}%`;
    }
  }

  function togglePlay() {
    if (videoEl.paused) videoEl.play().catch(() => {});
    else videoEl.pause();
    pokeActivity();
  }

  playBtn.addEventListener('click', togglePlay);
  videoEl.addEventListener('click', togglePlay);
  videoEl.addEventListener('dblclick', () => fsBtn.click());
  videoEl.addEventListener('play', syncPlayerUi);
  videoEl.addEventListener('pause', syncPlayerUi);
  videoEl.addEventListener('timeupdate', syncPlayerUi);
  videoEl.addEventListener('progress', syncPlayerUi);
  videoEl.addEventListener('loadedmetadata', syncPlayerUi);
  videoEl.addEventListener('volumechange', syncPlayerUi);

  muteBtn.addEventListener('click', () => {
    videoEl.muted = !videoEl.muted;
    syncPlayerUi();
  });
  volRange.addEventListener('input', () => {
    const v = Number(volRange.value);
    videoEl.muted = v === 0;
    videoEl.volume = v;
    if (dubEnabled) savedVolume = v;
  });

  seekWrap.addEventListener('click', (e) => {
    const rect = seekWrap.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    if (Number.isFinite(videoEl.duration) && videoEl.duration > 0) {
      videoEl.currentTime = ratio * videoEl.duration;
      syncPlayerUi();
    }
  });

  const SPEEDS = [1, 1.25, 1.5, 2, 0.75];
  speedBtn.addEventListener('click', () => {
    const cur = Number(videoEl.playbackRate) || 1;
    const idx = SPEEDS.indexOf(cur);
    const next = SPEEDS[(idx + 1) % SPEEDS.length];
    videoEl.playbackRate = next;
    speedBtn.textContent = `${next}x`;
  });

  window.addEventListener('keydown', (e) => {
    if (e.target && (['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target.tagName) || e.target.isContentEditable)) return;
    if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
      if (e.code === 'KeyC') {
        e.preventDefault();
        e.stopPropagation();
        subBtn.click();
        return;
      }
      if (e.code === 'KeyD') {
        e.preventDefault();
        e.stopPropagation();
        dubBtn.click();
        return;
      }
    }
    const key = e.key.toLowerCase();
    if (key === ' ' || key === 'k') {
      e.preventDefault();
      e.stopPropagation();
      togglePlay();
    } else if (key === 'arrowleft' || key === 'j') {
      e.preventDefault();
      e.stopPropagation();
      videoEl.currentTime = Math.max(0, videoEl.currentTime - (key === 'j' ? 10 : 5));
    } else if (key === 'arrowright' || key === 'l') {
      e.preventDefault();
      e.stopPropagation();
      videoEl.currentTime = Math.min(videoEl.duration || Infinity, videoEl.currentTime + (key === 'l' ? 10 : 5));
    } else if (key === 'm') {
      e.preventDefault();
      videoEl.muted = !videoEl.muted;
    } else if (key === 'f') {
      e.preventDefault();
      fsBtn.click();
    }
  }, true);

  playerWrap.focus();

  const { serverUrl } = await chrome.storage.local.get({
    serverUrl: 'http://127.0.0.1:4873',
  });
  const baseUrl = (serverUrl || 'http://127.0.0.1:4873').replace(/\/+$/, '');
  videoEl.src = `${baseUrl}/stream/${encodeURIComponent(videoId)}`;

  videoEl.addEventListener('playing', () => {
    setTimeout(() => {
      badgeEl.style.opacity = '0';
    }, 3500);
  });

  videoEl.addEventListener('error', () => {
    errorBox.classList.add('visible');
    errorSub.textContent = 'Stream failed. Check that YT Cloud Saver Desktop App is running and synced.';
  });

  // Auto-load cached subtitles if available so CC works right away
  try {
    const cached = await sendBg('GET_SUBTITLES', { videoId });
    if (cached?.cues?.length) {
      subBundle = cached;
      subMode = subStyle.defaultMode || 'bilingual';
      updateButtons();
      if (!syncRaf) syncRaf = requestAnimationFrame(tickSubtitles);
    }
  } catch {}
})();
