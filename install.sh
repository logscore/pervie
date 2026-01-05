#!/bin/sh
set -e

REPO="logscore/pervie"
BINARY="pervie"
INSTALL_DIR="/usr/local/bin"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux) OS="linux" ;;
  darwin) OS="macos" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64) ARCH="amd64" ;;
  arm64|aarch64) ARCH="arm64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "Fetching latest release..."
LATEST=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep "tag_name" | cut -d '"' -f 4)
if [ -z "$LATEST" ]; then
  echo "Error: Could not find latest release."
  exit 1
fi

URL="https://github.com/$REPO/releases/download/$LATEST/pervie-${OS}-${ARCH}.tar.gz"

echo "Downloading $URL..."
curl -sL "$URL" -o /tmp/pervie.tar.gz

tar -xzf /tmp/pervie.tar.gz -C /tmp/

# Use sudo only if the install directory is not writable
if [ -w "$INSTALL_DIR" ]; then
  mv /tmp/pervie "$INSTALL_DIR/$BINARY"
  chmod +x "$INSTALL_DIR/$BINARY"
else
  sudo mv /tmp/pervie "$INSTALL_DIR/$BINARY"
  sudo chmod +x "$INSTALL_DIR/$BINARY"
fi

rm /tmp/pervie.tar.gz
echo "Successfully installed $BINARY to path."
