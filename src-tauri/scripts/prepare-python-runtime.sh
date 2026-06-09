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
      MINGW*|MSYS*|CYGWIN*) os="windows" ;;
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
    windows) PLATFORM_TAG="${arch}-pc-windows-msvc" ;;
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

  echo "Extracting to $RUNTIME_DIR/macos/"
  rm -rf "$RUNTIME_DIR/macos"
  tar -xzf "$tarball" -C "$RUNTIME_DIR"
  local extracted_dir
  extracted_dir=$(ls -d "$RUNTIME_DIR"/python-install 2>/dev/null || ls -d "$RUNTIME_DIR"/python 2>/dev/null || echo "")
  if [[ -n "$extracted_dir" ]]; then
    mv "$extracted_dir" "$RUNTIME_DIR/macos"
  else
    mkdir -p "$RUNTIME_DIR/macos"
    shopt -s dotglob
    mv "$RUNTIME_DIR"/*/ "$RUNTIME_DIR/macos/" 2>/dev/null || true
    shopt -u dotglob
  fi
  echo "$PYTHON_VERSION-$PLATFORM_TAG" > "$marker"
}

# -- Install Dependencies ---------------------------------------
install_deps() {
  local python_bin
  if [[ "$PLATFORM_TAG" == *windows* ]]; then
    python_bin="$RUNTIME_DIR/windows/python/python.exe"
  else
    python_bin="$RUNTIME_DIR/macos/python/bin/python3"
  fi

  if [[ ! -x "$python_bin" ]]; then
    echo "Error: Cannot find python: $python_bin"
    exit 1
  fi

  echo "Upgrading pip..."
  "$python_bin" -m pip install --upgrade pip --quiet

  echo "Installing dependencies..."
  local pip_cmd=("$python_bin" -m pip install)
  if [[ -n "${PIP_INDEX_URL:-}" ]]; then
    pip_cmd+=("-i" "$PIP_INDEX_URL")
  fi

  if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
    echo "CI detected: Forcing CPU-only versions of torch/torchaudio..."
    "${pip_cmd[@]}" torch torchaudio --index-url https://download.pytorch.org/whl/cpu --no-cache-dir --quiet
  fi

  "${pip_cmd[@]}" -r "$REQUIREMENTS" --no-cache-dir --quiet

  echo "Aggressively cleaning up runtime..."
  find "$RUNTIME_DIR/macos" -type f \( -name "*.pdb" -o -name "*.lib" -o -name "*.a" -o -name "*.h" -o -name "*.cpp" \) -delete 2>/dev/null || true
  local unneeded_dirs=("__pycache__" "tests" "test" "Include" "share" "tcl" "tk" "idlelib" "ensurepip")
  for d in "${unneeded_dirs[@]}"; do
    find "$RUNTIME_DIR/macos" -type d -name "$d" -exec rm -rf {} + 2>/dev/null || true
  done
  find "$RUNTIME_DIR/macos" -type d \( -name "tests" -o -name "test" \) -exec rm -rf {} + 2>/dev/null || true
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
