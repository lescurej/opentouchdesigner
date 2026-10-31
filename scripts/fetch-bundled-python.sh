#!/usr/bin/env bash
set -euo pipefail

PBS_RELEASE="${OTD_PBS_RELEASE:-20251217}"
PYTHON_VERSION="${OTD_PYTHON_VERSION:-3.12.12}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
    echo "usage: fetch-bundled-python.sh <aarch64-apple-darwin|x86_64-apple-darwin>" >&2
    exit 1
}

TARGET="${1:-}"
[ -n "$TARGET" ] || usage

case "$TARGET" in
    aarch64-apple-darwin) ARCH=aarch64 ;;
    x86_64-apple-darwin) ARCH=x86_64 ;;
    *) usage ;;
esac

CACHE="$ROOT/.cache/python-build-standalone/$PBS_RELEASE/$TARGET"
PY_ROOT="$CACHE/python"
MARKER="$PY_ROOT/bin/python3"

if [ -x "$MARKER" ]; then
    echo "$PY_ROOT"
    exit 0
fi

mkdir -p "$CACHE"
TMP="$CACHE/.download"
rm -rf "$TMP"
mkdir -p "$TMP"

ASSET="cpython-${PYTHON_VERSION}+${PBS_RELEASE}-${TARGET}-install_only_stripped.tar.gz"
URL="https://github.com/astral-sh/python-build-standalone/releases/download/${PBS_RELEASE}/${ASSET}"

echo "==> downloading $ASSET" >&2
curl -fsSL "$URL" -o "$TMP/archive.tar.gz"
tar xzf "$TMP/archive.tar.gz" -C "$CACHE"
rm -rf "$TMP"

if [ ! -x "$MARKER" ]; then
    echo "!! expected $MARKER after extract" >&2
    exit 1
fi

echo "$PY_ROOT"
