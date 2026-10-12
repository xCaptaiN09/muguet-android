#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/env.sh"

if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
    mkdir -p "$MUGUET_TOOLS_DIR/downloads"
    installer="$MUGUET_TOOLS_DIR/downloads/rustup-init"
    base=https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
    curl --fail --location --retry 3 "$base" --output "$installer"
    curl --fail --location --retry 3 "$base.sha256" --output "$installer.sha256"
    expected="$(awk '{print $1}' "$installer.sha256")"
    actual="$(sha256sum "$installer")"
    actual="${actual%% *}"
    if [[ "$actual" != "$expected" ]]; then
        echo 'Rustup installer checksum mismatch.' >&2
        exit 1
    fi
    chmod +x "$installer"
    RUSTUP_INIT_SKIP_PATH_CHECK=yes "$installer" -y --no-modify-path --profile minimal --default-toolchain 1.99.0
fi

"$CARGO_HOME/bin/rustup" toolchain install 1.99.0 --profile minimal
"$CARGO_HOME/bin/rustup" target add aarch64-linux-android --toolchain 1.99.0
"$CARGO_HOME/bin/rustup" component add rustfmt clippy --toolchain 1.99.0
"$CARGO_HOME/bin/rustc" --version
