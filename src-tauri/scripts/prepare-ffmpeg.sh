#!/usr/bin/env bash
#
# 下载 FFmpeg 静态二进制文件并解压到 src-tauri/ffmpeg-runtime/
#
# 最终结构：
#   src-tauri/ffmpeg-runtime/
#     ├── macos/
#     │   ├── ffmpeg              (macOS)
#     │   └── ffprobe             (macOS)
#     └── windows/
#         ├── ffmpeg.exe          (Windows)
#         └── ffprobe.exe         (Windows)
#
set -euo pipefail

# 默认版本
FFMPEG_VERSION_MACOS="7.0.1"
FFMPEG_VERSION_LINUX="7.0.1"
FFMPEG_VERSION_WIN="7.0.1"

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RUNTIME_DIR="$( cd "$SCRIPT_DIR/.." && pwd )/ffmpeg-runtime"

# ── 平台检测 ─────────────────────────────────────────
detect_target() {
  local os="${TARGET_OS:-}"
  local arch="${TARGET_ARCH:-}"
  if [[ -z "$os" ]]; then
    case "$(uname -s)" in
      Darwin) os="macos" ;;
      Linux)  os="linux" ;;
      MINGW*|MSYS*|CYGWIN*) os="windows" ;;
      *) echo "❌ 未知平台 $(uname -s)"; exit 1 ;;
    esac
  fi
  if [[ -z "$arch" ]]; then
    case "$(uname -m)" in
      arm64|aarch64) arch="arm64" ;;
      x86_64|amd64)  arch="x64" ;;
      *) echo "❌ 未知架构 $(uname -m)"; exit 1 ;;
    esac
  fi

  echo "▸ 目标平台: $os/$arch"
  OS=$os
  ARCH=$arch
}

# ── 下载与解压 ───────────────────────────────────────
download_ffmpeg() {
  mkdir -p "$RUNTIME_DIR/.cache"
  
  if [[ "$OS" == "macos" ]]; then
    local ffmpeg_url=""
    local ffprobe_url=""
    local platform_dir="$RUNTIME_DIR/macos"
    mkdir -p "$platform_dir"

    if [[ "$ARCH" == "arm64" ]]; then
      # macOS ARM64 使用 Martin Riedl 的静态构建
      ffmpeg_url="https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip"
      ffprobe_url="https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffprobe.zip"
    else
      # macOS x64 使用 evermeet.cx 的静态构建
      ffmpeg_url="https://evermeet.cx/ffmpeg/getrelease/zip"
      ffprobe_url="https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip"
    fi

    if [[ ! -f "$platform_dir/ffmpeg" ]]; then
      echo "▸ 下载 FFmpeg (macOS/$ARCH)..."
      curl -L --fail --progress-bar -o "$RUNTIME_DIR/.cache/ffmpeg.zip" "$ffmpeg_url"
      echo "▸ 解压 FFmpeg..."
      unzip -o "$RUNTIME_DIR/.cache/ffmpeg.zip" -d "$platform_dir"
    fi

    if [[ ! -f "$platform_dir/ffprobe" ]]; then
      echo "▸ 下载 ffprobe (macOS/$ARCH)..."
      curl -L --fail --progress-bar -o "$RUNTIME_DIR/.cache/ffprobe.zip" "$ffprobe_url"
      echo "▸ 解压 ffprobe..."
      unzip -o "$RUNTIME_DIR/.cache/ffprobe.zip" -d "$platform_dir"
    fi

    # 移除 macOS 的隔离标记，否则无法运行二进制
    echo "▸ 移除隔离标记..."
    xattr -dr com.apple.quarantine "$platform_dir/ffmpeg" 2>/dev/null || true
    xattr -dr com.apple.quarantine "$platform_dir/ffprobe" 2>/dev/null || true

  elif [[ "$OS" == "linux" ]]; then
    local url=""
    if [[ "$ARCH" == "x64" ]]; then
      url="https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz"
    else
      url="https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-arm64-static.tar.xz"
    fi
    local tarball="$RUNTIME_DIR/.cache/ffmpeg-linux.tar.xz"
    
    if [[ ! -f "$tarball" ]]; then
      echo "▸ 下载 FFmpeg (Linux)..."
      curl -L --fail --progress-bar -o "$tarball" "$url"
    fi
    echo "▸ 解压到 $RUNTIME_DIR/linux/"
    local platform_dir="$RUNTIME_DIR/linux"
    mkdir -p "$platform_dir"
    tar -xJf "$tarball" -C "$platform_dir" --strip-components=1
    
  elif [[ "$OS" == "windows" ]]; then
    local url="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
    local tarball="$RUNTIME_DIR/.cache/ffmpeg-win.zip"
    local platform_dir="$RUNTIME_DIR/windows"

    mkdir -p "$platform_dir"

    if [[ ! -f "$tarball" ]]; then
      echo "▸ 下载 FFmpeg (Windows)..."
      curl -L --fail --progress-bar -o "$tarball" "$url"
    fi
    echo "▸ 解压到 $platform_dir/"
    local tmp_dir="$RUNTIME_DIR/tmp_ffmpeg"
    rm -rf "$tmp_dir"
    mkdir -p "$tmp_dir"
    unzip -o "$tarball" -d "$tmp_dir"
    # Move bin/*.exe to windows/
    if [[ -d "$tmp_dir"/*/bin ]]; then
      cp "$tmp_dir"/*/bin/*.exe "$platform_dir/" 2>/dev/null || true
    fi
    rm -rf "$tmp_dir"
    chmod +x "$platform_dir"/ffmpeg.exe "$platform_dir"/ffprobe.exe 2>/dev/null || true
  fi

  chmod +x "$RUNTIME_DIR"/macos/ffmpeg* "$RUNTIME_DIR"/macos/ffprobe* 2>/dev/null || true
}

# ── 汇报体积 ─────────────────────────────────────────
report_size() {
  echo ""
  echo "✅ FFmpeg 运行时准备完成"
  case "$(uname -s)" in
    Darwin)   ls -lh "$RUNTIME_DIR/macos/ffmpeg" "$RUNTIME_DIR/macos/ffprobe" 2>/dev/null ;;
    Linux)    ls -lh "$RUNTIME_DIR/linux/ffmpeg" "$RUNTIME_DIR/linux/ffprobe" 2>/dev/null ;;
    MINGW*|MSYS*|CYGWIN*) ls -lh "$RUNTIME_DIR/windows/ffmpeg.exe" "$RUNTIME_DIR/windows/ffprobe.exe" ;;
  esac
}

main() {
  detect_target
  download_ffmpeg
  report_size
}

main "$@"
