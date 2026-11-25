import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

// Ensure Homebrew paths are available when running under macOS LaunchAgent.
export const TOOL_ENV = {
  ...process.env,
  PATH: `/opt/homebrew/bin:/usr/local/bin:${process.env.PATH || '/usr/bin:/bin'}`,
};

async function hasBinary(name) {
  try {
    await execFileAsync('which', [name], { env: TOOL_ENV });
    return true;
  } catch {
    return false;
  }
}

export async function checkTools() {
  const [ytDlp, ffmpeg, rclone] = await Promise.all([
    hasBinary('yt-dlp'),
    hasBinary('ffmpeg'),
    hasBinary('rclone'),
  ]);
  return { ytDlp, ffmpeg, rclone };
}

export async function listRcloneRemotes() {
  try {
    const { stdout } = await execFileAsync('rclone', ['listremotes'], { env: TOOL_ENV });
    return stdout
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

export async function listRemoteDirs(remotePath) {
  if (!remotePath || !String(remotePath).includes(':')) {
    throw new Error('Invalid rclone path (expected format like remote:folder)');
  }
  const { stdout } = await execFileAsync(
    'rclone',
    ['lsjson', '--dirs-only', '--no-modtime', '--no-mimetype', String(remotePath)],
    { env: TOOL_ENV, maxBuffer: 4 * 1024 * 1024 },
  );
  const items = JSON.parse(stdout || '[]');
  return items
    .filter((item) => item.IsDir)
    .map((item) => item.Name)
    .sort((a, b) => a.localeCompare(b));
}

export async function createRemoteDir(remotePath) {
  if (!remotePath || !String(remotePath).includes(':')) {
    throw new Error('Invalid rclone path (expected format like remote:folder)');
  }
  await execFileAsync('rclone', ['mkdir', String(remotePath)], { env: TOOL_ENV });
  return { ok: true, path: String(remotePath) };
}
