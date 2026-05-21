#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

if [ -f "$ROOT_DIR/.env" ]; then
    set -a
    # shellcheck disable=SC1090
    source "$ROOT_DIR/.env"
    set +a
fi

VERSION="${1:-0.1.0}"
ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"

CLI_DIR="$ROOT_DIR/src-tauri/cli"
DIST_DIR="$ROOT_DIR/dist-cli"
PKG_ROOT="$DIST_DIR/package-root"

mkdir -p "$DIST_DIR"

pushd "$CLI_DIR" >/dev/null
cargo build --release
popd >/dev/null

rm -rf "$PKG_ROOT"
mkdir -p \
    "$PKG_ROOT/usr/local/bin" \
    "$PKG_ROOT/usr/share/doc/ezlogin-cli" \
    "$PKG_ROOT/usr/share/bash-completion/completions" \
    "$PKG_ROOT/usr/share/zsh/vendor-completions" \
    "$PKG_ROOT/usr/share/fish/vendor_completions.d" \
    "$PKG_ROOT/usr/share/man/man1"

install -m 0755 "$ROOT_DIR/src-tauri/target/release/ezlogin-cli" "$PKG_ROOT/usr/local/bin/ezlogin"

BIN="$PKG_ROOT/usr/local/bin/ezlogin"
"$BIN" completions bash > "$PKG_ROOT/usr/share/bash-completion/completions/ezlogin"
"$BIN" completions zsh  > "$PKG_ROOT/usr/share/zsh/vendor-completions/_ezlogin"
"$BIN" completions fish > "$PKG_ROOT/usr/share/fish/vendor_completions.d/ezlogin.fish"
"$BIN" man | gzip -9    > "$PKG_ROOT/usr/share/man/man1/ezlogin.1.gz"

cat > "$PKG_ROOT/usr/share/doc/ezlogin-cli/README" <<'EOF'
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

    cat > "$DEB_ROOT/DEBIAN/control" <<EOF
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
    echo "Created deb: $DEB_FILE"
else
    echo "dpkg-deb not found, skipped .deb packaging"
fi
