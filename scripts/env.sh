#!/usr/bin/env bash
# Source this file to select the project toolchains without changing system tools.
MUGUET_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
MUGUET_TOOLS_DIR="${MUGUET_TOOLS_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/muguet-android}"
export MUGUET_ROOT MUGUET_TOOLS_DIR
export CARGO_HOME="$MUGUET_TOOLS_DIR/cargo"
export RUSTUP_HOME="$MUGUET_TOOLS_DIR/rustup"
export RUSTUP_TOOLCHAIN="1.99.0"
export PATH="$CARGO_HOME/bin:$PATH"

if [[ -z "${JAVA_HOME:-}" && -d /usr/lib/jvm/java-21-openjdk ]]; then
    export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
fi
export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
