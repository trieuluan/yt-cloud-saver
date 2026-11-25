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
      case 'SYNC_CLOUD':
        return await apiFetch('/sync', { method: 'POST' });
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
