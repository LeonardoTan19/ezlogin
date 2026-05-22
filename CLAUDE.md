# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Desktop (Tauri + React) — dev server on port 1420
pnpm tauri dev

# Desktop with reduced memory usage (fewer parallel compiler jobs)
pnpm tauri:dev:lowmem

# Frontend only (Vite dev server)
pnpm dev

# Production build
pnpm tauri build

# CLI binary (run from src-tauri/)
cargo build --release          # output: src-tauri/target/release/ezlogin
cargo test -p ezlogin-core     # run only core tests (OCR + portal parsing)
cargo test --workspace         # run all Rust tests

# Ubuntu CLI packaging
./scripts/build-cli-ubuntu.sh <version>   # outputs tar.gz and .deb to dist-cli/

# Android APK (requires .env with SDK paths and signing credentials)
./scripts/build-android.sh

# Bump version across all manifests (package.json, Cargo.toml, tauri.conf.json)
./scripts/bump-version.sh <x.y.z>

# CI warm cache (trigger manually or pushes to main)
gh workflow run warm-cache.yml
```

## Architecture

### Workspace Structure

Three Rust crates in `src-tauri/` under a Cargo workspace:

| Crate | Purpose |
|---|---|
| `ezlogin-core` | Shared business logic: OCR, portal HTTP client, credential/login-options storage |
| `ezlogin-cli` | Standalone CLI binary (clap-based); depends on `ezlogin-core` |
| `ezlogin` (root) | Tauri desktop + Android app; wraps `ezlogin-core` as `#[tauri::command]` invocable from the React frontend |

### Key Data Flow

1. **Login**: React calls `portal_login_with_ocr` → Tauri lib (`src-tauri/src/lib.rs`) → `ezlogin_core::login_with_ocr` → `PortalClient` fetches captcha from portal (HTTPS), OCR engine (`rec.onnx` via tract-onnx) decodes it via greedy CTC, then submits credentials + captcha. Loop up to `maxLoginRetries` with exponential backoff (500ms → 4s max).

2. **Credentials storage**: Platform-conditional. On Android, passwords are AES-256-GCM encrypted with a key derived from `USER:HOSTNAME:EZLOGIN_SECRET_KEY`. On desktop, stored as plain JSON at `~/.config/ezlogin/credentials.json` with `0o600` permissions.

3. **Portal protocol**: The portal at `192.168.200.127:8445` uses XSRF-token-based session auth. `init_session()` warms the session by fetching auth.jsp, static assets, validCode endpoint, and POSTing config actions. On desktop, static fetches run concurrently via `tokio::join!`; on Android they're sequential. Android has an HTTP→HTTPS fallback for broken TLS on some devices.

4. **OCR pipeline**: `rec.onnx` (embedded via `include_bytes!`) → `tract-onnx` inference. Input: RGB image resized to height=48, width clamped to 320, normalized to [-1,1]. Output: greedy CTC beam search over `dict.txt` characters, with confidence averaging. Result filtered to at most 4 ASCII alphanumeric characters.

### Frontend

- **Framework**: React 19 + TypeScript + Vite + Tailwind CSS v4 + shadcn/ui (Radix primitives)
- **Entry**: `src/main.tsx` → `src/App.tsx`
- **Two views**: `LoginForm` and `SettingsPanel`, toggled via `view` state in App
- **State**: All state lives in `App` component; child components are purely presentational (prop-driven)
- **Tauri bridge**: `invoke()` calls from `@tauri-apps/api/core` — command names match the Rust function names in `src-tauri/src/lib.rs`

### CI (release.yml)

Triggered by `v*` tags. Three parallel jobs:
- **cli-ubuntu-amd64**: Compiles standalone CLI with `scripts/build-cli-ubuntu.sh`, uploads `.deb`
- **desktop-windows**: Uses `tauri-apps/tauri-action@v0` to build Windows installer and publish GitHub Release
- **android-aarch64**: Cross-compiles to `aarch64-linux-android` with NDK 27, produces signed APK

All three jobs converge in `publish-release` which uploads `.deb` and `.apk` artifacts to the release. Windows desktop artifacts are published directly by tauri-action.

## Testing

- Rust tests live alongside source (`#[cfg(test)] mod tests`)
- `ezlogin-core` tests cover: OCR recognition accuracy (sample images in `resources/`), portal login response parsing (success/failure classification, message extraction, statusCode edge cases for locked/wrong-credential accounts)
- No frontend test suite exists
