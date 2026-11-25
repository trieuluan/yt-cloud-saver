import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CONFIG_PATH = path.join(__dirname, 'config.json');

export const DEFAULT_CONFIG = {
  port: Number(process.env.PORT) || 4873,
  host: '127.0.0.1',
  rcloneRemote: process.env.RCLONE_REMOTE || 'gdrive:YouTube Saved',
  groupByChannel: true,
  quality: 'best', // 'best' | '1080' | '720'
  maxConcurrent: 2,
  keepLocal: false,
  tempDir: path.join(os.tmpdir(), 'yt-cloud-saver'),
};

export function loadConfig() {
  try {
    if (fs.existsSync(CONFIG_PATH)) {
      const raw = JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'));
      return { ...DEFAULT_CONFIG, ...raw };
    }
  } catch (err) {
    console.warn('[config] Failed to read config.json, using defaults:', err.message);
  }
  return { ...DEFAULT_CONFIG };
}

export function saveConfig(partial) {
  const current = loadConfig();
  const next = {
    ...current,
    ...(partial.rcloneRemote !== undefined ? { rcloneRemote: String(partial.rcloneRemote).trim() } : {}),
    ...(partial.groupByChannel !== undefined ? { groupByChannel: Boolean(partial.groupByChannel) } : {}),
    ...(partial.quality !== undefined ? { quality: String(partial.quality) } : {}),
    ...(partial.keepLocal !== undefined ? { keepLocal: Boolean(partial.keepLocal) } : {}),
    ...(partial.maxConcurrent !== undefined ? { maxConcurrent: Math.max(1, Math.min(4, Number(partial.maxConcurrent) || 2)) } : {}),
  };
  fs.writeFileSync(CONFIG_PATH, JSON.stringify(next, null, 2) + '\n', 'utf8');
  return next;
}
