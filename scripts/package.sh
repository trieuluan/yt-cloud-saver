#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$PROJECT_DIR/dist"
rm -f "$PROJECT_DIR/dist/yt-cloud-saver-extension.zip"
(cd "$PROJECT_DIR/extension" && zip -r "$PROJECT_DIR/dist/yt-cloud-saver-extension.zip" .)
echo "Created: $PROJECT_DIR/dist/yt-cloud-saver-extension.zip"
