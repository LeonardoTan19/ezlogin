#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

usage() {
    echo "Usage: $0 <version>"
    echo "  version  e.g. 1.2.0"
    exit 1
}

NEW_VERSION="${1:-}"
if [ -z "$NEW_VERSION" ]; then
    usage
fi

# Validate semver format
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: version must be in X.Y.Z format, got: $NEW_VERSION" >&2
    exit 1
fi

echo "Bumping to $NEW_VERSION ..."

# package.json
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$NEW_VERSION\"/" "$ROOT_DIR/package.json"

# tauri.conf.json
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$NEW_VERSION\"/" "$ROOT_DIR/src-tauri/tauri.conf.json"

# Cargo.toml files — only replace the [package] section's version line
for toml in \
    "$ROOT_DIR/src-tauri/Cargo.toml" \
    "$ROOT_DIR/src-tauri/core/Cargo.toml" \
    "$ROOT_DIR/src-tauri/cli/Cargo.toml"; do
    # Use awk to only replace the first occurrence (in [package] section)
    awk -v ver="$NEW_VERSION" '
        /^\[/ { in_pkg = (/^\[package\]/) }
        in_pkg && /^version = / { sub(/"[^"]*"/, "\"" ver "\""); in_pkg = 0 }
        { print }
    ' "$toml" > "$toml.tmp" && mv "$toml.tmp" "$toml"
done

# Regenerate Cargo.lock
(cd "$ROOT_DIR/src-tauri" && cargo check --workspace -q 2>&1 | grep -E "^error" || true)

echo "Done. Files updated:"
grep -rn "\"version\".*$NEW_VERSION\|^version = \"$NEW_VERSION\"" \
    "$ROOT_DIR/package.json" \
    "$ROOT_DIR/src-tauri/tauri.conf.json" \
    "$ROOT_DIR/src-tauri/Cargo.toml" \
    "$ROOT_DIR/src-tauri/core/Cargo.toml" \
    "$ROOT_DIR/src-tauri/cli/Cargo.toml"
