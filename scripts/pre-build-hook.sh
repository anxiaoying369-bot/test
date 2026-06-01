#!/bin/bash
# scripts/pre-build-hook.sh
# Tauri 构建前 Hook：根据当前平台动态选择 runtime resources
# 由 package.json 的 beforeBuildCommand 调用

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# --- Patch tauri.conf.json resources ---
CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"
BACKUP="$CONF.bak"

# 检测平台
HOST_OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$HOST_OS" in
  darwin*)  RUNTIME_PLATFORM="macos" ;;
  mingw*|msys*|cygwin*|windows*) RUNTIME_PLATFORM="windows" ;;
  linux*)  RUNTIME_PLATFORM="linux" ;;
  *)
    echo "[pre-build-hook] Unknown platform: $HOST_OS, skipping resource patch"
    exit 0
    ;;
esac

echo "[pre-build-hook] Platform: $HOST_OS → runtime: $RUNTIME_PLATFORM"

# Patch
python3 - <<EOF
import json, shutil

CONF = "$CONF"

with open(CONF, "r", encoding="utf-8") as f:
    conf = json.load(f)

resources = conf["bundle"]["resources"]
replaced = []
for r in resources:
    if "python-runtime/macos" in r:
        replaced.append(r.replace("python-runtime/macos", f"python-runtime/{RUNTIME_PLATFORM}"))
    elif "ffmpeg-runtime/macos" in r:
        replaced.append(r.replace("ffmpeg-runtime/macos", f"ffmpeg-runtime/{RUNTIME_PLATFORM}"))
    else:
        replaced.append(r)

conf["bundle"]["resources"] = replaced

shutil.copy(CONF, CONF + ".bak")
with open(CONF, "w", encoding="utf-8") as f:
    json.dump(conf, f, indent=2, ensure_ascii=False)

print(f"[pre-build-hook] Resources patched → python-runtime/{RUNTIME_PLATFORM}, ffmpeg-runtime/{RUNTIME_PLATFORM}")
EOF

# Trap: restore original config after build (success or failure)
RESTORE_MARKER="__RESTORED__"
restore_config() {
    if [ -f "$BACKUP" ] && [ ! -f "$CONF.$RESTORE_MARKER" ]; then
        mv "$BACKUP" "$CONF"
        touch "$CONF.$RESTORE_MARKER"
        # Clean up marker file after a short delay
        rm -f "$CONF.$RESTORE_MARKER" 2>/dev/null || true
        echo "[pre-build-hook] tauri.conf.json restored"
    fi
}

trap restore_config EXIT INT TERM

# Restore original config immediately after patching (tauri CLI reads config once at start)
if [ -f "$BACKUP" ]; then
    cp "$CONF" "$CONF.patched"  # keep patched version for tauri
    cp "$BACKUP" "$CONF"        # restore for safety, tauri reads patched separately
fi

echo "[pre-build-hook] Proceeding with build..."