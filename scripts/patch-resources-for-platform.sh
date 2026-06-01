#!/bin/bash
# scripts/patch-resources-for-platform.sh
# 在 tauri build 前根据 HOST 平台选择正确的 runtime resources
# 用法: bash scripts/patch-resources-for-platform.sh

set -e

CONF="src-tauri/tauri.conf.json"
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')

case "$PLATFORM" in
  darwin*)  RUNTIME_PLATFORM="macos" ;;
  mingw*|msys*|cygwin*|windows*) RUNTIME_PLATFORM="windows" ;;
  linux*)  RUNTIME_PLATFORM="linux" ;;
  *)
    echo "Unknown platform: $PLATFORM"
    exit 1
    ;;
esac

echo "[patch-resources] Detected platform: $PLATFORM → runtime: $RUNTIME_PLATFORM"

# Patch tauri.conf.json — 替换 resources 中的平台路径
# 使用 Python 因为它跨平台且无需额外依赖
python3 - <<EOF
import json, sys, shutil, os

CONF = "src-tauri/tauri.conf.json"
PLATFORM = "$PLATFORM"
RUNTIME = "$RUNTIME_PLATFORM"

with open(CONF, "r", encoding="utf-8") as f:
    conf = json.load(f)

resources = conf["bundle"]["resources"]

# 替换 macos → 当前平台
replaced = []
for r in resources:
    if "python-runtime/macos" in r:
        replaced.append(r.replace("python-runtime/macos", f"python-runtime/{RUNTIME}"))
    elif "ffmpeg-runtime/macos" in r:
        replaced.append(r.replace("ffmpeg-runtime/macos", f"ffmpeg-runtime/{RUNTIME}"))
    else:
        replaced.append(r)

conf["bundle"]["resources"] = replaced

backup = CONF + ".bak"
shutil.copy(CONF, backup)

with open(CONF, "w", encoding="utf-8") as f:
    json.dump(conf, f, indent=2, ensure_ascii=False)

print(f"[patch-resources] Patched → python-runtime/{RUNTIME}, ffmpeg-runtime/{RUNTIME}")

# Restore after build completes (trap handled by caller)
EOF

echo "[patch-resources] tauri.conf.json patched for $RUNTIME_PLATFORM platform"