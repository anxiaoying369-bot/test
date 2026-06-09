#!/usr/bin/env bash
# Download Node.js portable and put into src-tauri/node-runtime/macos/
set -euo pipefail

NODE_VERSION="v20.15.1"
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then PLATFORM="darwin-x64"; else PLATFORM="darwin-arm64"; fi

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RUNTIME_DIR="$( cd "$SCRIPT_DIR/.." && pwd )/node-runtime"
PLATFORM_DIR="$RUNTIME_DIR/macos"

if [ -f "$PLATFORM_DIR/bin/node" ]; then
    echo "Node.js already exists in $PLATFORM_DIR, skipping."
    exit 0
fi

mkdir -p "$RUNTIME_DIR/.cache"
mkdir -p "$PLATFORM_DIR"

URL="https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-$PLATFORM.tar.gz"
TARBALL="$RUNTIME_DIR/.cache/node-macos.tar.gz"

if [ ! -f "$TARBALL" ]; then
    echo "Downloading Node.js ($PLATFORM)..."
    curl -L -o "$TARBALL" "$URL"
fi

echo "Extracting to $PLATFORM_DIR"
tar -xzf "$TARBALL" -C "$PLATFORM_DIR" --strip-components=1

echo "Node.js runtime prepared successfully."
