import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadConfig } from './config.js';
import { TOOL_ENV } from './tools.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const JOBS_PATH = path.join(__dirname, 'jobs.json');
const MAX_HISTORY = 100;

const ACTIVE_STATUSES = new Set(['queued', 'downloading', 'uploading']);

function sanitizeFolderName(name) {
  return String(name || '')
    .replace(/[<>:"/\\|?*\x00-\x1F]/g, '_')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^\.+|\.+$/g, '')
    .slice(0, 100);
}

function joinRemotePath(baseRemote, subFolder) {
  const cleanBase = String(baseRemote || '').replace(/\/+$/, '');
  if (!subFolder) return cleanBase;
  return cleanBase.endsWith(':') ? `${cleanBase}${subFolder}` : `${cleanBase}/${subFolder}`;
}

function buildFormatSelector(quality) {
  if (quality === '1080') {
    return 'bv*[height<=1080][ext=mp4]+ba[ext=m4a]/bv*[height<=1080]+ba/b[height<=1080]/b';
  }
  if (quality === '720') {
    return 'bv*[height<=720][ext=mp4]+ba[ext=m4a]/bv*[height<=720]+ba/b[height<=720]/b';
  }
  return 'bv*[ext=mp4]+ba[ext=m4a]/bv*+ba/b';
}

export class JobManager {
  constructor() {
    this.jobs = this.#loadHistory();
    this.processes = new Map();
  }

  #loadHistory() {
    try {
      if (!fs.existsSync(JOBS_PATH)) return [];
      const list = JSON.parse(fs.readFileSync(JOBS_PATH, 'utf8'));
      if (!Array.isArray(list)) return [];
      // Any job left active from a previous server crash is marked interrupted.
      return list.map((job) =>
        ACTIVE_STATUSES.has(job.status)
          ? { ...job, status: 'error', error: 'Server restarted while job was active', updatedAt: new Date().toISOString() }
          : job,
      );
    } catch {
      return [];
    }
  }

  #persist() {
    try {
      const trimmed = this.jobs.slice(0, MAX_HISTORY);
      fs.writeFileSync(JOBS_PATH, JSON.stringify(trimmed, null, 2) + '\n', 'utf8');
    } catch (err) {
      console.warn('[jobs] Failed to persist jobs.json:', err.message);
    }
  }

  list() {
    return this.jobs;
  }

  get(id) {
    return this.jobs.find((j) => j.id === id) || null;
  }

  enqueue({ url, videoId, title, channelName, timestamp = 0, quality }) {
    const existing = this.jobs.find(
      (j) => (videoId && j.videoId === videoId) && ACTIVE_STATUSES.has(j.status),
    );
    if (existing) return existing;

    const config = loadConfig();
    const channelFolder = config.groupByChannel ? sanitizeFolderName(channelName) : '';
    const targetRemote = joinRemotePath(config.rcloneRemote, channelFolder);
    const now = new Date().toISOString();
    const job = {
      id: crypto.randomUUID(),
      url,
      videoId: videoId || null,
      title: title || url,
      channelName: channelName || '',
      timestamp: Math.max(0, Math.floor(Number(timestamp) || 0)),
      quality: quality || config.quality || 'best',
      remote: targetRemote,
      status: 'queued',
      progress: 0,
      stageText: 'Queued',
      fileName: null,
      error: null,
      createdAt: now,
      updatedAt: now,
    };

    this.jobs.unshift(job);
    this.#persist();
    this.#pump();
    return job;
  }

  cancel(id) {
    const job = this.get(id);
    if (!job) return null;
    if (!ACTIVE_STATUSES.has(job.status)) return job;

    job.status = 'cancelled';
    job.stageText = 'Cancelled';
    job.updatedAt = new Date().toISOString();

    const child = this.processes.get(id);
    if (child) {
      child.kill('SIGTERM');
      this.processes.delete(id);
    }

    this.#cleanupJobDir(id);
    this.#persist();
    this.#pump();
    return job;
  }

  remove(id) {
    const job = this.get(id);
    if (!job) return false;
    if (ACTIVE_STATUSES.has(job.status)) {
      this.cancel(id);
    }
    this.jobs = this.jobs.filter((j) => j.id !== id);
    this.#persist();
    return true;
  }

  #pump() {
    const config = loadConfig();
    const runningCount = this.jobs.filter((j) => j.status === 'downloading' || j.status === 'uploading').length;
    if (runningCount >= config.maxConcurrent) return;

    const next = [...this.jobs].reverse().find((j) => j.status === 'queued');
    if (!next) return;

    this.#runJob(next, config);
    this.#pump();
  }

  #updateJob(job, patch) {
    Object.assign(job, patch, { updatedAt: new Date().toISOString() });
    this.#persist();
  }

  #jobDir(id) {
    const config = loadConfig();
    return path.join(config.tempDir, id);
  }

  #cleanupJobDir(id) {
    const dir = this.#jobDir(id);
    try {
      fs.rmSync(dir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  }

  async #runJob(job, config) {
    const jobDir = this.#jobDir(job.id);
    fs.mkdirSync(jobDir, { recursive: true });

    try {
      this.#updateJob(job, { status: 'downloading', progress: 0, stageText: 'Starting yt-dlp...' });
      const filePath = await this.#downloadVideo(job, jobDir);
      if (job.status === 'cancelled') return;

      const fileName = path.basename(filePath);
      this.#updateJob(job, {
        status: 'uploading',
        progress: 0,
        fileName,
        stageText: `Uploading to ${job.remote}...`,
      });

      await this.#uploadToCloud(job, filePath);
      if (job.status === 'cancelled') return;

      this.#updateJob(job, {
        status: 'done',
        progress: 100,
        stageText: `Saved to ${job.remote}`,
      });
    } catch (err) {
      if (job.status !== 'cancelled') {
        this.#updateJob(job, {
          status: 'error',
          error: err.message || 'Unknown error',
          stageText: 'Failed',
        });
      }
    } finally {
      this.processes.delete(job.id);
      if (!config.keepLocal || job.status !== 'done') {
        this.#cleanupJobDir(job.id);
      }
      this.#pump();
    }
  }

  #downloadVideo(job, jobDir) {
    return new Promise((resolve, reject) => {
      const outputTemplate = path.join(jobDir, '%(title).120B [%(id)s].%(ext)s');
      const args = [
        '--newline',
        '--no-playlist',
        '--windows-filenames',
        '-f',
        buildFormatSelector(job.quality),
        '--merge-output-format',
        'mp4',
        '-o',
        outputTemplate,
        job.url,
      ];

      const child = spawn('yt-dlp', args, { env: TOOL_ENV });
      this.processes.set(job.id, child);

      let stderrTail = '';
      let buffer = '';

      const handleLine = (line) => {
        const match = line.match(/\[download\]\s+(\d+(?:\.\d+)?)%/);
        if (match) {
          const pct = Math.min(99, Math.round(Number(match[1])));
          if (pct !== job.progress) {
            this.#updateJob(job, { progress: pct, stageText: `Downloading ${pct}%` });
          }
        } else if (line.includes('[Merger]') || line.includes('Merging formats')) {
          this.#updateJob(job, { progress: 99, stageText: 'Merging video & audio...' });
        }
      };

      child.stdout.on('data', (chunk) => {
        buffer += chunk.toString();
        const lines = buffer.split(/\r?\n/);
        buffer = lines.pop() || '';
        for (const line of lines) handleLine(line);
      });

      child.stderr.on('data', (chunk) => {
        stderrTail = (stderrTail + chunk.toString()).slice(-1500);
      });

      child.on('error', (err) => {
        reject(new Error(`Failed to start yt-dlp: ${err.message}`));
      });

      child.on('close', (code) => {
        if (job.status === 'cancelled') return reject(new Error('Cancelled'));
        if (code !== 0) {
          return reject(new Error(stderrTail.trim() || `yt-dlp exited with code ${code}`));
        }
        const files = fs
          .readdirSync(jobDir)
          .filter((name) => !name.endsWith('.part') && !name.endsWith('.ytdl'));
        if (!files.length) {
          return reject(new Error('Download finished but output file was not found'));
        }
        const mp4 = files.find((name) => name.endsWith('.mp4')) || files[0];
        resolve(path.join(jobDir, mp4));
      });
    });
  }

  #uploadToCloud(job, filePath) {
    return new Promise((resolve, reject) => {
      const args = [
        'copy',
        filePath,
        job.remote,
        '--stats=1s',
        '--stats-one-line',
        '-v',
      ];

      const child = spawn('rclone', args, { env: TOOL_ENV });
      this.processes.set(job.id, child);

      let stderrTail = '';

      child.stderr.on('data', (chunk) => {
        const text = chunk.toString();
        stderrTail = (stderrTail + text).slice(-1500);
        const match = text.match(/(\d+)%/);
        if (match) {
          const pct = Math.min(99, Math.round(Number(match[1])));
          if (pct !== job.progress) {
            this.#updateJob(job, { progress: pct, stageText: `Uploading ${pct}%` });
          }
        }
      });

      child.on('error', (err) => {
        reject(new Error(`Failed to start rclone: ${err.message}`));
      });

      child.on('close', (code) => {
        if (job.status === 'cancelled') return reject(new Error('Cancelled'));
        if (code !== 0) {
          return reject(new Error(stderrTail.trim() || `rclone exited with code ${code}`));
        }
        resolve();
      });
    });
  }
}
