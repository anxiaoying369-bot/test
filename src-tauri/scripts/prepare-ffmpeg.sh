#!/usr/bin/env bash
#
# Download FFmpeg static binaries and extract to src-tauri/ffmpeg-runtime/
#
# Final structure:
#   src-tauri/ffmpeg-runtime/
#     ├── macos/
#     │   ├── ffmpeg              (macOS)
#     │   └── ffprobe             (macOS)
#     └── windows/
#         ├── ffmpeg.exe          (Windows)
#         └── ffprobe.exe         (Windows)
#
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RUNTIME_DIR="$( cd "$SCRIPT_DIR/.." && pwd )/ffmpeg-runtime"

# -- Platform Detection -----------------------------------------
detect_target() {
  local os="${TARGET_OS:-}"
  local arch="${TARGET_ARCH:-}"
  if [[ -z "$os" ]]; then
    case "$(uname -s)" in
      Darwin) os="macos" ;;
      Linux)  os="linux" ;;
      MINGW*|MSYS*|CYGWIN*) os="windows" ;;
      *) echo "Error: Unknown platform $(uname -s)"; exit 1 ;;
    esac
  fi
  if [[ -z "$arch" ]]; then
    case "$(uname -m)" in
      arm64|aarch64) arch="arm64" ;;
      x86_64|amd64)  arch="x64" ;;
      *) echo "Error: Unknown architecture $(uname -m)"; exit 1 ;;
    esac
  fi

  echo "Target Platform: $os/$arch"
  OS=$os
  ARCH=$arch
}

# -- Download and Extract ---------------------------------------
download_ffmpeg() {
  local platform_dir="$RUNTIME_DIR/macos"
  if [[ "$OS" == "windows" ]]; then platform_dir="$RUNTIME_DIR/windows"; fi
  
  if [[ -f "$platform_dir/ffmpeg" ]] || [[ -f "$platform_dir/ffmpeg.exe" ]]; then
    echo "FFmpeg binaries already exist in $platform_dir, skipping."
    return 0
  fi

  mkdir -p "$RUNTIME_DIR/.cache"
  
  if [[ "$OS" == "macos" ]]; then
    local ffmpeg_url=""
    local ffprobe_url=""
    local platform_dir="$RUNTIME_DIR/macos"
    mkdir -p "$platform_dir"

    if [[ "$ARCH" == "arm64" ]]; then
      ffmpeg_url="https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip"
      ffprobe_url="https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffprobe.zip"
    else
      ffmpeg_url="https://evermeet.cx/ffmpeg/getrelease/zip"
      ffprobe_url="https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip"
    fi

    if [[ ! -f "$platform_dir/ffmpeg" ]]; then
      echo "Downloading FFmpeg (macOS/$ARCH)..."
      curl -L --fail --progress-bar -o "$RUNTIME_DIR/.cache/ffmpeg.zip" "$ffmpeg_url"
      echo "Extracting FFmpeg..."
      unzip -o "$RUNTIME_DIR/.cache/ffmpeg.zip" -d "$platform_dir"
    fi

    if [[ ! -f "$platform_dir/ffprobe" ]]; then
      echo "Downloading ffprobe (macOS/$ARCH)..."
      curl -L --fail --progress-bar -o "$RUNTIME_DIR/.cache/ffprobe.zip" "$ffprobe_url"
      echo "Extracting ffprobe..."
      unzip -o "$RUNTIME_DIR/.cache/ffprobe.zip" -d "$platform_dir"
    fi

    echo "Removing quarantine flags..."
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
      echo "Downloading FFmpeg (Linux)..."
      curl -L --fail --progress-bar -o "$tarball" "$url"
    fi
    echo "Extracting to $RUNTIME_DIR/linux/"
    local platform_dir="$RUNTIME_DIR/linux"
    mkdir -p "$platform_dir"
    tar -xJf "$tarball" -C "$platform_dir" --strip-components=1
    
  elif [[ "$OS" == "windows" ]]; then
    local url="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
    local tarball="$RUNTIME_DIR/.cache/ffmpeg-win.zip"
    local platform_dir="$RUNTIME_DIR/windows"

    mkdir -p "$platform_dir"

    if [[ ! -f "$tarball" ]]; then
      echo "Downloading FFmpeg (Windows)..."
      curl -L --fail --progress-bar -o "$tarball" "$url"
    fi
    echo "Extracting to $platform_dir/"
    local tmp_dir="$RUNTIME_DIR/tmp_ffmpeg"
    rm -rf "$tmp_dir"
    mkdir -p "$tmp_dir"
    unzip -o "$tarball" -d "$tmp_dir"
    if [[ -d "$tmp_dir"/*/bin ]]; then
      cp "$tmp_dir"/*/bin/*.exe "$platform_dir/" 2>/dev/null || true
    fi
    rm -rf "$tmp_dir"
    chmod +x "$platform_dir"/ffmpeg.exe "$platform_dir"/ffprobe.exe 2>/dev/null || true
  fi

  chmod +x "$RUNTIME_DIR"/macos/ffmpeg* "$RUNTIME_DIR"/macos/ffprobe* 2>/dev/null || true
}

# -- Report Size ------------------------------------------------
report_size() {
  echo ""
  echo "FFmpeg runtime prepared successfully."
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
