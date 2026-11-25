#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLIST_PATH="$HOME/Library/LaunchAgents/com.louis.yt-cloud-saver.plist"
NODE_BIN="$(command -v node || echo /Users/louis/.nvm/versions/node/v24.2.0/bin/node)"

echo "==> Checking Homebrew dependencies (yt-dlp, ffmpeg, rclone)..."
for pkg in yt-dlp ffmpeg rclone; do
  if command -v "$pkg" >/dev/null 2>&1; then
    echo "  ✓ $pkg is installed ($(command -v "$pkg"))"
  else
    echo "  -> Installing $pkg..."
    brew install "$pkg"
  fi
done

echo ""
echo "==> Configured rclone remotes:"
rclone listremotes || true

echo ""
if [[ "${1:-}" == "--launch-agent" ]]; then
  mkdir -p "$HOME/Library/LaunchAgents"
  cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.louis.yt-cloud-saver</string>
  <key>ProgramArguments</key>
  <array>
    <string>${NODE_BIN}</string>
    <string>${PROJECT_DIR}/server/index.js</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>${PROJECT_DIR}/server/server.log</string>
  <key>StandardErrorPath</key>
  <string>${PROJECT_DIR}/server/server.log</string>
</dict>
</plist>
EOF
  launchctl unload "$PLIST_PATH" 2>/dev/null || true
  launchctl load "$PLIST_PATH"
  echo "✓ LaunchAgent installed and started: $PLIST_PATH"
else
  echo "Tip: Run '$0 --launch-agent' to auto-start the local server on macOS login."
fi
