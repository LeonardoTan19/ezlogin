#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAURI_DIR="$ROOT_DIR/src-tauri"
CLI_DIR="$TAURI_DIR/cli"
DIST_DIR="$ROOT_DIR/dist-cli"
PKG_ROOT="$DIST_DIR/package-root"

VERSION="${1:-0.1.0}"
ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"

mkdir -p "$DIST_DIR"

pushd "$CLI_DIR" >/dev/null
cargo build --release
popd >/dev/null

rm -rf "$PKG_ROOT"
mkdir -p "$PKG_ROOT/usr/local/bin"
mkdir -p "$PKG_ROOT/usr/share/doc/ezlogin-cli"

install -m 0755 "$CLI_DIR/target/release/ezlogin-cli" "$PKG_ROOT/usr/local/bin/ezlogin"
cat >"$PKG_ROOT/usr/share/doc/ezlogin-cli/README" <<'EOF'
ezlogin CLI

Usage:
  ezlogin init --account <ACCOUNT> --password <PASSWORD>
  ezlogin login
EOF

TARBALL="$DIST_DIR/ezlogin-cli_${VERSION}_linux_${ARCH}.tar.gz"
tar -czf "$TARBALL" -C "$PKG_ROOT" .

echo "Created tarball: $TARBALL"

if command -v dpkg-deb >/dev/null 2>&1; then
  DEB_ROOT="$DIST_DIR/deb-root"
  rm -rf "$DEB_ROOT"
  mkdir -p "$DEB_ROOT/DEBIAN"
  cp -a "$PKG_ROOT/." "$DEB_ROOT/"

  cat >"$DEB_ROOT/DEBIAN/control" <<EOF
Package: ezlogin-cli
Version: $VERSION
Section: net
Priority: optional
Architecture: $ARCH
Maintainer: ezlogin
Description: EZLogin command line tool for Ubuntu
EOF

  DEB_FILE="$DIST_DIR/ezlogin-cli_${VERSION}_${ARCH}.deb"
  dpkg-deb --build "$DEB_ROOT" "$DEB_FILE" >/dev/null
  echo "Created deb package: $DEB_FILE"
else
  echo "dpkg-deb not found, skipped .deb packaging"
fi
