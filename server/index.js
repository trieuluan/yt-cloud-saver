import http from 'node:http';
import { loadConfig, saveConfig } from './config.js';
import { JobManager } from './jobs.js';
import { checkTools, createRemoteDir, listRcloneRemotes, listRemoteDirs } from './tools.js';

const jobs = new JobManager();

function sendJson(res, statusCode, payload) {
  res.writeHead(statusCode, {
    'Content-Type': 'application/json; charset=utf-8',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, DELETE, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  });
  res.end(JSON.stringify(payload));
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let raw = '';
    req.on('data', (chunk) => {
      raw += chunk;
      if (raw.length > 1024 * 64) {
        reject(new Error('Request body too large'));
        req.destroy();
      }
    });
    req.on('end', () => {
      if (!raw) return resolve({});
      try {
        resolve(JSON.parse(raw));
      } catch {
        reject(new Error('Invalid JSON body'));
      }
    });
    req.on('error', reject);
  });
}

function isValidYouTubeUrl(value) {
  try {
    const u = new URL(value);
    return (
      u.protocol === 'https:' &&
      (u.hostname === 'www.youtube.com' ||
        u.hostname === 'youtube.com' ||
        u.hostname === 'm.youtube.com' ||
        u.hostname === 'youtu.be')
    );
  } catch {
    return false;
  }
}

const server = http.createServer(async (req, res) => {
  if (req.method === 'OPTIONS') {
    return sendJson(res, 204, {});
  }

  const reqUrl = new URL(req.url || '/', 'http://127.0.0.1');
  const pathname = reqUrl.pathname;

  try {
    if (req.method === 'GET' && pathname === '/health') {
      const [tools, remotes] = await Promise.all([checkTools(), listRcloneRemotes()]);
      return sendJson(res, 200, {
        ok: true,
        tools,
        remotes,
        config: loadConfig(),
      });
    }

    if (req.method === 'GET' && pathname === '/config') {
      return sendJson(res, 200, { config: loadConfig() });
    }

    if (req.method === 'POST' && pathname === '/config') {
      const body = await readBody(req);
      const config = saveConfig(body);
      return sendJson(res, 200, { config });
    }

    if (req.method === 'GET' && pathname === '/remotes/dirs') {
      const remotePath = reqUrl.searchParams.get('path') || '';
      const dirs = await listRemoteDirs(remotePath);
      return sendJson(res, 200, { path: remotePath, dirs });
    }

    if (req.method === 'POST' && pathname === '/remotes/mkdir') {
      const body = await readBody(req);
      const result = await createRemoteDir(body.path);
      return sendJson(res, 200, result);
    }

    if (req.method === 'GET' && pathname === '/jobs') {
      return sendJson(res, 200, { jobs: jobs.list() });
    }

    if (req.method === 'POST' && pathname === '/save') {
      const body = await readBody(req);
      if (!body.url || !isValidYouTubeUrl(body.url)) {
        return sendJson(res, 400, { error: 'Valid YouTube URL is required' });
      }
      const job = jobs.enqueue({
        url: body.url,
        videoId: body.videoId,
        title: body.title,
        channelName: body.channelName,
        timestamp: body.timestamp,
        quality: body.quality,
      });
      return sendJson(res, 202, { job });
    }

    const cancelMatch = pathname.match(/^\/jobs\/([^/]+)\/cancel$/);
    if (req.method === 'POST' && cancelMatch) {
      const job = jobs.cancel(cancelMatch[1]);
      if (!job) return sendJson(res, 404, { error: 'Job not found' });
      return sendJson(res, 200, { job });
    }

    const jobMatch = pathname.match(/^\/jobs\/([^/]+)$/);
    if (jobMatch) {
      if (req.method === 'GET') {
        const job = jobs.get(jobMatch[1]);
        if (!job) return sendJson(res, 404, { error: 'Job not found' });
        return sendJson(res, 200, { job });
      }
      if (req.method === 'DELETE') {
        const removed = jobs.remove(jobMatch[1]);
        if (!removed) return sendJson(res, 404, { error: 'Job not found' });
        return sendJson(res, 200, { ok: true });
      }
    }

    return sendJson(res, 404, { error: 'Not found' });
  } catch (err) {
    return sendJson(res, 500, { error: err.message || 'Internal server error' });
  }
});

const config = loadConfig();
server.listen(config.port, config.host, () => {
  console.log(`[yt-cloud-saver] Server listening on http://${config.host}:${config.port}`);
  console.log(`[yt-cloud-saver] Default rclone remote: ${config.rcloneRemote}`);
});
