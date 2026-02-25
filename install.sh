#!/bin/sh
# Install script for Code HUD
# Usage: curl -fsSL https://raw.githubusercontent.com/Tidemarks-AI/Code-HUD/main/install.sh | sh

set -eu

REPO="Tidemarks-AI/Code-HUD"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

get_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)
      case "$arch" in
        x86_64|amd64) echo "x86_64-unknown-linux-musl" ;;
        aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64|amd64) echo "x86_64-apple-darwin" ;;
        aarch64|arm64) echo "aarch64-apple-darwin" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
      esac
      ;;
    *) echo "Unsupported OS: $os" >&2; exit 1 ;;
  esac
}

get_latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
}

verify_checksum() {
  tarball="$1"
  checksum_file="$2"
  os="$(uname -s)"

  case "$os" in
    Linux)
      sha256sum -c "$checksum_file" || {
        echo "ERROR: Checksum verification failed" >&2
        exit 1
      }
      ;;
    Darwin)
      shasum -a 256 -c "$checksum_file" || {
        echo "ERROR: Checksum verification failed" >&2
        exit 1
      }
      ;;
    *)
      echo "ERROR: Checksum verification not supported on $os" >&2
      exit 1
      ;;
  esac
}

main() {
  target="$(get_target)"
  version="${VERSION:-$(get_latest_version)}"

  echo "Installing codehud ${version} for ${target}..."

  base_url="https://github.com/${REPO}/releases/download/${version}"
  tarball_name="codehud-${version}-${target}.tar.gz"
  tarball_url="${base_url}/${tarball_name}"
  checksums_url="${base_url}/codehud-${version}-checksums.sha256"
  
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  echo "Downloading ${tarball_name}..."
  curl -fsSL "$tarball_url" -o "$tmpdir/$tarball_name"
  
  echo "Downloading checksums..."
  curl -fsSL "$checksums_url" -o "$tmpdir/checksums.sha256"
  
  echo "Verifying checksum..."
  cd "$tmpdir"
  grep "$tarball_name" checksums.sha256 > "$tarball_name.sha256"
  verify_checksum "$tarball_name" "$tarball_name.sha256"
  
  echo "Extracting..."
  tar xzf "$tarball_name"
  
  chmod +x "$tmpdir/codehud"

  if [ -w "$INSTALL_DIR" ]; then
    mv "$tmpdir/codehud" "$INSTALL_DIR/codehud"
  else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "$tmpdir/codehud" "$INSTALL_DIR/codehud"
  fi

  echo "Installed codehud to ${INSTALL_DIR}/codehud"
}

main
