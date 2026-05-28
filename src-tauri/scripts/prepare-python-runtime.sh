#!/usr/bin/env bash
#
# 下载 python-build-standalone（来自 astral-sh），解压到 src-tauri/python-runtime/，
# 用它的 pip 安装 requirements.txt。最终结构：
#   src-tauri/python-runtime/
#     ├── python/                 ← Tauri 会打包整个目录
#     │   ├── bin/python3         (macOS/Linux)
#     │   ├── python.exe          (Windows)
#     │   └── lib/python3.11/site-packages/  ← pip 装的包都在这
#     └── .version                ← 标记当前安装的 Python 版本
#
# 用法：
#   ./prepare-python-runtime.sh                  # 自动检测当前平台
#   TARGET_OS=windows ./prepare-python-runtime.sh  # 强制为指定平台准备
#
set -euo pipefail

PYTHON_VERSION="3.11.10"
RELEASE_TAG="20241016"   # 升级时同步检查 https://github.com/astral-sh/python-build-standalone/releases

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RUNTIME_DIR="$( cd "$SCRIPT_DIR/.." && pwd )/python-runtime"
REPO_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
REQUIREMENTS="$REPO_ROOT/requirements.txt"

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
      arm64|aarch64) arch="aarch64" ;;
      x86_64|amd64)  arch="x86_64" ;;
      *) echo "❌ 未知架构 $(uname -m)"; exit 1 ;;
    esac
  fi

  case "$os" in
    macos)   PLATFORM_TAG="${arch}-apple-darwin" ;;
    linux)   PLATFORM_TAG="${arch}-unknown-linux-gnu" ;;
    windows) PLATFORM_TAG="${arch}-pc-windows-msvc" ;;
    *) echo "❌ 不支持的 OS: $os"; exit 1 ;;
  esac

  echo "▸ 目标平台: $os/$arch  →  $PLATFORM_TAG"
}

# ── 下载与解压 ───────────────────────────────────────
download_python() {
  local url="https://github.com/astral-sh/python-build-standalone/releases/download/${RELEASE_TAG}/cpython-${PYTHON_VERSION}+${RELEASE_TAG}-${PLATFORM_TAG}-install_only.tar.gz"
  local tarball="$RUNTIME_DIR/.cache/python-${PYTHON_VERSION}-${PLATFORM_TAG}.tar.gz"
  local marker="$RUNTIME_DIR/.version"

  if [[ -f "$marker" ]] && grep -q "$PYTHON_VERSION-$PLATFORM_TAG" "$marker"; then
    echo "✓ Python $PYTHON_VERSION ($PLATFORM_TAG) 已存在，跳过下载"
    return 0
  fi

  mkdir -p "$RUNTIME_DIR/.cache"
  if [[ ! -f "$tarball" ]]; then
    echo "▸ 下载 python-build-standalone..."
    echo "  $url"
    curl -L --fail --progress-bar -o "$tarball.tmp" "$url"
    mv "$tarball.tmp" "$tarball"
  fi

  echo "▸ 解压到 $RUNTIME_DIR/python/"
  rm -rf "$RUNTIME_DIR/python"
  tar -xzf "$tarball" -C "$RUNTIME_DIR"
  # python-build-standalone 解压后顶层就是 python/
  echo "$PYTHON_VERSION-$PLATFORM_TAG" > "$marker"
}

# ── 安装依赖 ─────────────────────────────────────────
install_deps() {
  local python_bin
  if [[ "$PLATFORM_TAG" == *windows* ]]; then
    python_bin="$RUNTIME_DIR/python/python.exe"
  else
    python_bin="$RUNTIME_DIR/python/bin/python3"
  fi

  if [[ ! -x "$python_bin" ]]; then
    echo "❌ 找不到 python: $python_bin"
    ls -la "$RUNTIME_DIR/python/" 2>&1 | head -20
    exit 1
  fi

  if [[ ! -f "$REQUIREMENTS" ]]; then
    echo "❌ 找不到 requirements.txt: $REQUIREMENTS"
    exit 1
  fi

  echo "▸ 升级 pip..."
  "$python_bin" -m pip install --upgrade pip --quiet

  echo "▸ 安装依赖（来自 ${REQUIREMENTS}）..."
  "$python_bin" -m pip install -r "$REQUIREMENTS"

  echo "▸ 清理 __pycache__ 和 .pyc 减小体积..."
  find "$RUNTIME_DIR/python" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
  find "$RUNTIME_DIR/python" -name "*.pyc" -delete 2>/dev/null || true
  find "$RUNTIME_DIR/python" -name "*.pyo" -delete 2>/dev/null || true
  # 删除 pip/setuptools 测试目录可省 ~30MB
  find "$RUNTIME_DIR/python" -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true
  find "$RUNTIME_DIR/python" -type d -name "test"  -exec rm -rf {} + 2>/dev/null || true
}

# ── 汇报体积 ─────────────────────────────────────────
report_size() {
  local size
  size=$(du -sh "$RUNTIME_DIR/python" 2>/dev/null | awk '{print $1}')
  echo ""
  echo "✅ Python 运行时准备完成"
  echo "   位置: $RUNTIME_DIR/python"
  echo "   体积: ${size:-未知}"
  echo ""
  echo "下一步: 运行 npm run tauri build"
}

main() {
  detect_target
  download_python
  install_deps
  report_size
}

main "$@"
