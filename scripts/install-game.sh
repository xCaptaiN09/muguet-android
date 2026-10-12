#!/usr/bin/env bash
# Install this development APK and verified local game content through ADB.
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/env.sh"
package=dev.xcaptain.muguet
apk="${MUGUET_ANDROID_APK:-$MUGUET_ROOT/artifacts/muguet-android-game-debug.apk}"
content="${MUGUET_ANDROID_CONTENT:-$HOME/.local/share/muguet-android/prepared/full-game-v1}"
code="${MUGUET_ANDROID_CODE_ARTIFACT:-$HOME/.local/share/muguet-android/compiled/full-game-v1/game-android.cwasm}"
expected="${MUGUET_ANDROID_CODE_SHA256:-d9b0341855b8a2197d8aaeb81e7bc8ae71dd6785c1168e632dc77729888631e9}"
external="/sdcard/Android/data/$package/files"

[[ -f "$apk" && -f "$code" && -f "$content/data/manifest.json" ]] || {
    echo "Missing APK, compiler artifact, or prepared content manifest" >&2; exit 1;
}
[[ -f "$content/b/8b0b5899ed/shaders/index.json" ]] || {
    echo "Prepared content has no selected shader index" >&2; exit 1;
}
actual="$(sha256sum "$code")"; actual="${actual%% *}"
[[ "$actual" == "$expected" ]] || { echo "Compiler artifact differs from trusted digest" >&2; exit 1; }
devices="$(adb devices | awk '$2 == "device" {print $1}')"
[[ "$(wc -w <<<"$devices")" -eq 1 ]] || { echo "Connect exactly one authorized ADB device" >&2; exit 1; }
adb shell true >/dev/null

# Measure the destination before starting any large transfer. A currently
# installed app can be updated in place without erasing its external files.
available_kib="$(adb shell df -Pk /sdcard | tr -d '\r' | awk 'NR==2 {print $4}')"
[[ "$available_kib" =~ ^[0-9]+$ ]] || { echo "Cannot read phone free space" >&2; exit 1; }
if ! adb shell pm path "$package" | tr -d '\r' | rg -q '^package:'; then
    echo "App package is not installed; installing APK first"
fi
adb install -r "$apk"
adb shell pm path "$package" | tr -d '\r' | rg -q '^package:' || {
    echo "Installed app package is not visible" >&2; exit 1;
}
adb shell mkdir -p "$external/game" "$external/game-code"

if adb shell test -f "$external/game/data/manifest.json" &&
   adb shell test -f "$external/game/b/8b0b5899ed/shaders/index.json"; then
    echo "Game content already staged; skipping bulk transfer"
else
    required_kib="$(du -sk "$content" | awk '{print $1}')"
    [[ "$available_kib" -gt "$((required_kib + 1048576))" ]] || {
        echo "Insufficient phone storage for prepared content" >&2; exit 1;
    }
    echo "Transferring prepared content to $external/game"
    adb push "$content/." "$external/game/"
fi

remote_code="$external/game-code/game-android.cwasm"
remote_hash=""
if adb shell test -f "$remote_code"; then
    remote_hash="$(adb shell sha256sum "$remote_code" | tr -d '\r' | awk '{print $1}')"
fi
if [[ "$remote_hash" == "$expected" ]]; then
    echo "Trusted code staging already matches; skipping transfer"
else
    code_kib="$(du -k "$code" | awk '{print $1}')"
    [[ "$available_kib" -gt "$((code_kib + 524288))" ]] || {
        echo "Insufficient phone storage for compiled code" >&2; exit 1;
    }
    adb push "$code" "$remote_code.part"
    adb shell mv "$remote_code.part" "$remote_code"
    remote_hash="$(adb shell sha256sum "$remote_code" | tr -d '\r' | awk '{print $1}')"
    [[ "$remote_hash" == "$expected" ]] || { echo "Phone staging digest mismatch" >&2; exit 1; }
fi

# Some debug ROMs permit run-as, while this Realme ROM denies it via SELinux.
# The app itself performs an atomic private copy from the same staging location
# on a background worker, then native verifies the baked digest before load.
if adb shell run-as "$package" id >/dev/null 2>&1; then
    if adb shell run-as "$package" mkdir -p files/game-code &&
       adb shell run-as "$package" cp "$remote_code" files/game-code/game-android.cwasm; then
        echo "Private code copied with run-as"
    else
        echo "run-as copy unavailable; GameActivity will copy staged code into private storage"
    fi
else
    echo "run-as unavailable; GameActivity will copy staged code into private storage"
fi
echo "Install ready. Launch the game from the app; this script does not start it."
