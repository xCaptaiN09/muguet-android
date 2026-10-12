#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/env.sh"
cd "$MUGUET_ROOT/android"
bash ./gradlew --no-daemon :app:assembleDebug
mkdir -p "$MUGUET_ROOT/artifacts"
cp app/build/outputs/apk/debug/app-debug.apk "$MUGUET_ROOT/artifacts/muguet-android-debug.apk"
sha256sum "$MUGUET_ROOT/artifacts/muguet-android-debug.apk"
