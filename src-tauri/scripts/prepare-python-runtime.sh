#!/usr/bin/env bash
#
# Download python-build-standalone (from astral-sh), extract to src-tauri/python-runtime/
#
set -euo pipefail

PYTHON_VERSION="3.11.10"
RELEASE_TAG="20241016"

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RUNTIME_DIR="$( cd "$SCRIPT_DIR/.." && pwd )/python-runtime"
REPO_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
REQUIREMENTS="$REPO_ROOT/requirements.txt"

# -- Platform Detection -----------------------------------------
detect_target() {
  local os="${TARGET_OS:-}"
  local arch="${TARGET_ARCH:-}"
  if [[ -z "$os" ]]; then
    case "$(uname -s)" in
      Darwin) os="macos" ;;
      Linux)  os="linux" ;;
      *) echo "Error: Unknown platform $(uname -s)"; exit 1 ;;
    esac
  fi
  if [[ -z "$arch" ]]; then
    case "$(uname -m)" in
      arm64|aarch64) arch="aarch64" ;;
      x86_64|amd64)  arch="x86_64" ;;
      *) echo "Error: Unknown architecture $(uname -m)"; exit 1 ;;
    esac
  fi

  case "$os" in
    macos)   PLATFORM_TAG="${arch}-apple-darwin" ;;
    linux)   PLATFORM_TAG="${arch}-unknown-linux-gnu" ;;
    *) echo "Error: Unsupported OS: $os"; exit 1 ;;
  esac

  echo "Target Platform: $os/$arch  ->  $PLATFORM_TAG"
}

# -- Download and Extract ---------------------------------------
download_python() {
  local url="https://github.com/astral-sh/python-build-standalone/releases/download/${RELEASE_TAG}/cpython-${PYTHON_VERSION}+${RELEASE_TAG}-${PLATFORM_TAG}-install_only.tar.gz"
  local tarball="$RUNTIME_DIR/.cache/python-${PYTHON_VERSION}-${PLATFORM_TAG}.tar.gz"
  local marker="$RUNTIME_DIR/.version"

  if [[ -f "$marker" ]] && grep -q "$PYTHON_VERSION-$PLATFORM_TAG" "$marker"; then
    echo "Python $PYTHON_VERSION ($PLATFORM_TAG) already exists, skipping download."
    return 0
  fi

  mkdir -p "$RUNTIME_DIR/.cache"
  if [[ ! -f "$tarball" ]]; then
    echo "Downloading python-build-standalone..."
    echo "  $url"
    curl -L --fail --progress-bar -o "$tarball.tmp" "$url"
    mv "$tarball.tmp" "$tarball"
  fi

  echo "Extracting Python..."
  local temp_extract_dir="$RUNTIME_DIR/tmp_extract"
  rm -rf "$temp_extract_dir"
  mkdir -p "$temp_extract_dir"
  tar -xzf "$tarball" -C "$temp_extract_dir"

  local platform_dir="$RUNTIME_DIR/macos"
  rm -rf "$platform_dir"
  mkdir -p "$platform_dir"

  local extracted
  extracted=$(ls -d "$temp_extract_dir"/python-install 2>/dev/null || ls -d "$temp_extract_dir"/python 2>/dev/null || echo "")
  
  if [[ -n "$extracted" ]]; then
    mv "$extracted" "$platform_dir/python"
  else
    mkdir -p "$platform_dir/python"
    mv "$temp_extract_dir"/* "$platform_dir/python/"
  fi
  
  rm -rf "$temp_extract_dir"
  echo "$PYTHON_VERSION-$PLATFORM_TAG" > "$marker"
}

# -- Install Dependencies ---------------------------------------
install_deps() {
  local python_bin="$RUNTIME_DIR/macos/python/bin/python3"

  if [[ ! -x "$python_bin" ]]; then
    echo "Error: Cannot find python: $python_bin"
    exit 1
  fi

  echo "Upgrading pip..."
  "$python_bin" -m pip install --upgrade pip --quiet

  echo "Installing dependencies..."
  local pip_cmd=("$python_bin" -m pip install --no-cache-dir --quiet)
  if [[ -n "${PIP_INDEX_URL:-}" ]]; then
    pip_cmd+=("-i" "$PIP_INDEX_URL")
  fi

  # 注意：STT 已从 funasr+torch 切到 sherpa-onnx（纯 ONNX，无 torch），
  # 不再安装 torch，避免内置 Python 体积过大导致 Windows NSIS 打包溢出。
  "${pip_cmd[@]}" -r "$REQUIREMENTS"

  echo "Aggressively cleaning up runtime..."
  # 删除开发文件
  find "$RUNTIME_DIR/macos" -type f \( -name "*.pdb" -o -name "*.lib" -o -name "*.a" -o -name "*.h" -o -name "*.cpp" -o -name "*.c" -o -name "*.pyi" \) -delete 2>/dev/null || true
  
  # 删除不必要的文件夹
  local unneeded_dirs=("__pycache__" "tests" "test" "Include" "share" "tcl" "tk" "idlelib" "ensurepip" "doc" "docs")
  for d in "${unneeded_dirs[@]}"; do
    find "$RUNTIME_DIR/macos" -type d -name "$d" -exec rm -rf {} + 2>/dev/null || true
  done

  find "$RUNTIME_DIR/macos" -type d \( -name "tests" -o -name "test" -o -name "data" \) -exec rm -rf {} + 2>/dev/null || true

  # strip 原生扩展(.so/.dylib)的符号以缩小体积，并 ad-hoc 重签名
  # （macOS 上 strip 会让代码签名失效→加载时被系统杀，必须重新签名）。
  # 全程 `|| true`，任何失败都不影响构建。
  echo "Stripping + re-signing native libs..."
  find "$RUNTIME_DIR/macos" \( -name "*.so" -o -name "*.dylib" \) -exec strip -x {} \; 2>/dev/null || true
  find "$RUNTIME_DIR/macos" \( -name "*.so" -o -name "*.dylib" \) -exec codesign --force --sign - {} \; 2>/dev/null || true
}

report_size() {
  local size
  size=$(du -sh "$RUNTIME_DIR/macos" 2>/dev/null | awk '{print $1}')
  echo "Python runtime size: ${size:-Unknown}"
}

main() {
  detect_target
  download_python
  install_deps
  report_size
}

main "$@"
