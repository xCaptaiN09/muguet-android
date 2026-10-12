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
export CARGO_BUILD_JOBS=2

# The digest is embedded in Rust at compile time. A present compiler artifact
# determines the pin; a missing artifact still permits foundation diagnostics,
# while the native launch boundary rejects its missing pin.
artifact="${MUGUET_ANDROID_CODE_ARTIFACT:-$HOME/.local/share/muguet-android/compiled/full-game-v1/game-android.cwasm}"
if [[ -f "$artifact" ]]; then
    actual="$(sha256sum "$artifact")"
    actual="${actual%% *}"
    if [[ -n "${MUGUET_ANDROID_CODE_SHA256:-}" && "$MUGUET_ANDROID_CODE_SHA256" != "$actual" ]]; then
        echo "Trusted code digest does not match compiler artifact: $artifact" >&2
        exit 1
    fi
    export MUGUET_ANDROID_CODE_SHA256="$actual"
    echo "Embedding game code SHA-256: $actual ($artifact)"
elif [[ -n "${MUGUET_ANDROID_CODE_SHA256:-}" ]]; then
    echo "Compiler artifact missing; refusing supplied pin without artifact: $artifact" >&2
    exit 1
else
    unset MUGUET_ANDROID_CODE_SHA256
    echo "No compiler artifact found; game launch will be disabled" >&2
fi

cd "$MUGUET_ROOT"
cargo build --locked --release --target aarch64-linux-android -p muguet-android-native --lib
output="${1:-$MUGUET_ROOT/android/app/build/generated/rustJni}"
mkdir -p "$output/arm64-v8a"
cp "$MUGUET_ROOT/target/aarch64-linux-android/release/libmuguet_android.so" "$output/arm64-v8a/"
