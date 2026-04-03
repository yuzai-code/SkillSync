#!/bin/sh
# SkillSync installer — detects OS/architecture, downloads prebuilt binary from GitHub Releases.
# Usage: curl -fsSL https://raw.githubusercontent.com/OWNER/skillsync/main/install.sh | sh
set -e

REPO="OWNER/skillsync"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="skillsync"

# --- Detect OS ---
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# --- Map architecture ---
case "$ARCH" in
  x86_64|amd64)  ARCH="x86_64" ;;
  aarch64|arm64)  ARCH="aarch64" ;;
  *)
    echo "Error: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

# --- Map OS to Rust target triple ---
case "$OS" in
  darwin) TARGET="${ARCH}-apple-darwin" ;;
  linux)  TARGET="${ARCH}-unknown-linux-gnu" ;;
  *)
    echo "Error: unsupported OS: $OS" >&2
    exit 1
    ;;
esac

echo "Detected platform: $TARGET"

# --- Resolve latest version ---
if [ -z "$VERSION" ]; then
  VERSION=$(curl -sI "https://github.com/$REPO/releases/latest" \
    | grep -i '^location:' \
    | sed 's|.*/tag/||' \
    | tr -d '\r\n')
  if [ -z "$VERSION" ]; then
    echo "Error: could not determine latest version. Set VERSION env var manually." >&2
    exit 1
  fi
fi

echo "Installing $BINARY_NAME $VERSION for $TARGET..."

# --- Download binary ---
URL="https://github.com/$REPO/releases/download/$VERSION/$BINARY_NAME-$TARGET"
TMPFILE=$(mktemp)
HTTP_CODE=$(curl -sL -o "$TMPFILE" -w "%{http_code}" "$URL")

if [ "$HTTP_CODE" != "200" ]; then
  rm -f "$TMPFILE"
  echo "Error: download failed (HTTP $HTTP_CODE)" >&2
  echo "URL: $URL" >&2
  exit 1
fi

chmod +x "$TMPFILE"

# --- Install ---
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMPFILE" "$INSTALL_DIR/$BINARY_NAME"
else
  echo "Need sudo to install to $INSTALL_DIR"
  sudo mv "$TMPFILE" "$INSTALL_DIR/$BINARY_NAME"
fi

echo "$BINARY_NAME installed to $INSTALL_DIR/$BINARY_NAME"

# --- Verify ---
if command -v "$BINARY_NAME" >/dev/null 2>&1; then
  "$BINARY_NAME" --version
else
  echo "Note: $INSTALL_DIR may not be in your PATH. Add it with:"
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi
