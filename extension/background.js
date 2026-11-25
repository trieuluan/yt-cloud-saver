const DEFAULT_SERVER_URL = 'http://127.0.0.1:4873';

async function getServerUrl() {
  const { serverUrl } = await chrome.storage.local.get({ serverUrl: DEFAULT_SERVER_URL });
  return (serverUrl || DEFAULT_SERVER_URL).replace(/\/+$/, '');
}

async function apiFetch(path, options = {}) {
  const baseUrl = await getServerUrl();
  const res = await fetch(`${baseUrl}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers || {}),
    },
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(data.error || `HTTP ${res.status}`);
  }
  return data;
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  (async () => {
    switch (msg?.type) {
      case 'HEALTH':
        return await apiFetch('/health');
      case 'LIST_JOBS':
        return await apiFetch('/jobs');
      case 'GET_JOB':
        return await apiFetch(`/jobs/${encodeURIComponent(msg.id)}`);
      case 'SAVE_VIDEO':
        return await apiFetch('/save', {
          method: 'POST',
          body: JSON.stringify(msg.payload),
        });
      case 'CANCEL_JOB':
        return await apiFetch(`/jobs/${encodeURIComponent(msg.id)}/cancel`, {
          method: 'POST',
        });
      case 'DELETE_JOB':
        return await apiFetch(`/jobs/${encodeURIComponent(msg.id)}`, {
          method: 'DELETE',
        });
      case 'SAVE_CONFIG':
        return await apiFetch('/config', {
          method: 'POST',
          body: JSON.stringify(msg.payload),
        });
      case 'LIST_DIRS':
        return await apiFetch(`/remotes/dirs?path=${encodeURIComponent(msg.path || '')}`);
      case 'CREATE_DIR':
        return await apiFetch('/remotes/mkdir', {
          method: 'POST',
          body: JSON.stringify({ path: msg.path }),
        });
      case 'SHOW_APP':
        return await apiFetch('/show', { method: 'POST' });
      case 'GET_SAVED_IDS':
        return await apiFetch('/saved-ids');
      case 'DELETE_SAVED':
        return await apiFetch(`/saved/${encodeURIComponent(msg.videoId)}`, {
          method: 'DELETE',
        });
      case 'OPEN_CLOUD_PLAYER': {
        const url = chrome.runtime.getURL(`player.html?videoId=${encodeURIComponent(msg.videoId)}`);
        await chrome.tabs.create({ url });
        return { ok: true };
      }
      case 'SYNC_CLOUD':
        return await apiFetch('/sync', { method: 'POST' });
      case 'TRANSLATE_VIDEO':
        return await apiFetch('/translate', {
          method: 'POST',
          body: JSON.stringify(msg.payload),
        });
      case 'GET_SUBTITLES':
        return await apiFetch(`/subtitles/${encodeURIComponent(msg.videoId)}`);
      case 'PREPARE_TTS':
        return await apiFetch('/tts/prepare', {
          method: 'POST',
          body: JSON.stringify({
            texts: msg.texts || [],
            voice: msg.voice || 'vi-VN-HoaiMyNeural',
            rate: msg.rate || '+15%',
          }),
        });
      case 'SYNTH_TTS': {
        const baseUrl = await getServerUrl();
        const params = new URLSearchParams({
          text: msg.text || '',
          voice: msg.voice || 'vi-VN-HoaiMyNeural',
          rate: msg.rate || '+0%',
        });
        const res = await fetch(`${baseUrl}/tts?${params.toString()}`);
        if (!res.ok) {
          const err = await res.json().catch(() => ({}));
          throw new Error(err.error || `TTS HTTP ${res.status}`);
        }
        const buf = await res.arrayBuffer();
        const bytes = new Uint8Array(buf);
        let binary = '';
        const chunkSize = 0x8000;
        for (let i = 0; i < bytes.length; i += chunkSize) {
          binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
        }
        return { audioUrl: `data:audio/mpeg;base64,${btoa(binary)}` };
      }
      case 'GET_SERVER_URL':
        return { serverUrl: await getServerUrl() };
      case 'SET_SERVER_URL': {
        const next = (msg.serverUrl || DEFAULT_SERVER_URL).trim().replace(/\/+$/, '');
        await chrome.storage.local.set({ serverUrl: next });
        return { serverUrl: next };
      }
      default:
        throw new Error(`Unknown message type: ${msg?.type}`);
    }
  })()
    .then((data) => sendResponse({ ok: true, data }))
    .catch((err) => sendResponse({ ok: false, error: err.message || 'Request failed' }));

  return true;
});
