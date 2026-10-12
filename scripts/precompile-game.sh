#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/env.sh"
source_file="${1:-$HOME/.local/share/muguet-android/game-files/full-game-v1/mirror/playgta5.com/b/8b0b5899ed/game.wasm}"
output="${2:-$HOME/.local/share/muguet-android/compiled/full-game-v1/game-android.cwasm}"
[[ -f "$source_file" ]] || { echo "Missing source Wasm: $source_file" >&2; exit 1; }
mkdir -p "$(dirname -- "$output")"
export CARGO_BUILD_JOBS=2
cd "$MUGUET_ROOT"
cargo build --locked --release -p muguet-engine --features compiler --bin precompile-android
"$MUGUET_ROOT/target/release/precompile-android" "$source_file" "$output"
sha256sum "$output"
