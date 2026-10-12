#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/env.sh"

llvm="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
linker="$llvm/aarch64-linux-android29-clang"
if [[ ! -x "$linker" ]]; then
    echo "Missing Android NDK: $ANDROID_NDK_HOME" >&2
    echo "Install ndk;28.2.13676358 using the Android SDK manager." >&2
    exit 1
fi
if [[ ! -x "$CARGO_HOME/bin/cargo" ]]; then
    echo 'Run bash scripts/setup-rust.sh first.' >&2
    exit 1
fi

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$linker"
export CC_aarch64_linux_android="$linker"
export AR_aarch64_linux_android="$llvm/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS='-C link-arg=-Wl,-z,max-page-size=16384'

cd "$MUGUET_ROOT"
cargo build --locked --release --target aarch64-linux-android -p muguet-android-native --lib
output="${1:-$MUGUET_ROOT/android/app/build/generated/rustJni}"
mkdir -p "$output/arm64-v8a"
cp "$MUGUET_ROOT/target/aarch64-linux-android/release/libmuguet_android.so" "$output/arm64-v8a/"
