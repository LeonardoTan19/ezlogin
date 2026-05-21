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

: "${ANDROID_HOME:?ANDROID_HOME is not set. Copy .env.example to .env and configure it.}"
: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME is not set. Copy .env.example to .env and configure it.}"

export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
export NDK_HOME="${NDK_HOME:-$ANDROID_NDK_HOME}"

NDK_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
export PATH="$NDK_BIN:$PATH"
export CC_aarch64_linux_android="$NDK_BIN/aarch64-linux-android24-clang"
export CXX_aarch64_linux_android="$NDK_BIN/aarch64-linux-android24-clang++"
export AR_aarch64_linux_android="$NDK_BIN/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_BIN/aarch64-linux-android24-clang"

cd "$ROOT_DIR"
pnpm tauri android build --apk --target aarch64

APK="$(find src-tauri/gen/android/app/build/outputs/apk -type f -name '*.apk' | grep release | head -n 1 || true)"
if [ -n "$APK" ]; then
    echo "APK: $APK"
fi
