(async () => {
  const params = new URLSearchParams(location.search);
  const videoId = params.get('videoId');
  const videoEl = document.getElementById('video');
  const badgeEl = document.getElementById('badge');
  const errorBox = document.getElementById('errorBox');
  const errorSub = document.getElementById('errorSub');

  if (!videoId) {
    errorBox.classList.add('visible');
    errorSub.textContent = 'Missing videoId.';
    return;
  }

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
})();
