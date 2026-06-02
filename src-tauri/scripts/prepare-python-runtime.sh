#!/usr/bin/env bash
#
# Download python-build-standalone (from astral-sh), extract to src-tauri/python-runtime/
# Final structure:
#   src-tauri/python-runtime/
#     ├── macos/
#     │   ├── bin/python3         (macOS)
#     │   ├── include/            (macOS)
#     │   └── lib/python3.11/site-packages/  <- pip packages here
#     └── windows/
#         ├── python.exe        (Windows)
#         ├── python311.dll
#         └── lib/python3.11/site-packages/   <- pip packages here
#
# Usage:
#   ./prepare-python-runtime.sh                  # auto detect platform
#   TARGET_OS=windows ./prepare-python-runtime.sh  # force specified platform
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
    # fallback
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
    python_bin="$RUNTIME_DIR/windows/python.exe"
  else
    python_bin="$RUNTIME_DIR/macos/bin/python3"
  fi

  if [[ ! -x "$python_bin" ]]; then
    echo "Error: Cannot find python: $python_bin"
    ls -la "$RUNTIME_DIR/macos/" 2>&1 | head -20
    exit 1
  fi

  if [[ ! -f "$REQUIREMENTS" ]]; then
    echo "Error: Cannot find requirements.txt: $REQUIREMENTS"
    exit 1
  fi

  echo "Upgrading pip..."
  "$python_bin" -m pip install --upgrade pip --quiet

  echo "Installing dependencies (from ${REQUIREMENTS})..."
  "$python_bin" -m pip install -r "$REQUIREMENTS"

  echo "Cleaning __pycache__ and .pyc to reduce size..."
  find "$RUNTIME_DIR/macos" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
  find "$RUNTIME_DIR/macos" -name "*.pyc" -delete 2>/dev/null || true
  find "$RUNTIME_DIR/macos" -name "*.pyo" -delete 2>/dev/null || true
  find "$RUNTIME_DIR/macos" -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true
  find "$RUNTIME_DIR/macos" -type d -name "test"  -exec rm -rf {} + 2>/dev/null || true
}

# -- Report Size ------------------------------------------------
report_size() {
  local size
  size=$(du -sh "$RUNTIME_DIR/macos" 2>/dev/null | awk '{print $1}')
  echo ""
  echo "Python runtime prepared successfully."
  echo "   Path: $RUNTIME_DIR/macos"
  echo "   Size: ${size:-Unknown}"
  echo ""
  echo "Next step: Run npm run tauri build"
}

main() {
  detect_target
  download_python
  install_deps
  report_size
}

main "$@"
