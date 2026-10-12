# Experimental Android game host

The launcher retains its runtime and Vulkan diagnostic controls. **Start local game** opens
an unexported landscape activity in `dev.xcaptain.muguet:game`. Each process accepts one
960×432 game session. Leaving or backgrounding the activity pauses the engine, releases
held controls, exports final status, and ends only `:game`. Reopening starts a fresh process;
resuming the same game session is not supported in this milestone. Surface presentation
counts record requests, not proof of a visible or interactive game.

## Prepare on the PC

The default verified source is
`~/.local/share/muguet-android/game-files/full-game-v1/mirror/playgta5.com`.
The separate prepared tree is
`~/.local/share/muguet-android/prepared/full-game-v1`. Run:

```sh
python3 scripts/prepare-game.py
bash scripts/precompile-game.sh
bash scripts/build-apk.sh
```

Preparation hardlinks unchanged files where possible, omits any path component named
`env_test` or `verityradio` without regard to case, and writes filtered manifest/DLC
copies in the new tree. The verified source files are never edited. An existing valid
output is reused; incomplete or differing output is rejected. The compiler reads the
original `b/8b0b5899ed/game.wasm` and writes
`~/.local/share/muguet-android/compiled/full-game-v1/game-android.cwasm` plus its
`.sha256` sidecar. The APK build hashes the actual compiler artifact and embeds that
digest in native Rust using `MUGUET_ANDROID_CODE_SHA256` and `option_env!`. Without a
valid embedded digest, game launch fails while diagnostics remain available. The
installed artifact is verified against that embedded digest before Wasmtime
deserialization; an adjacent phone checksum is never trusted.

The development installer checks ADB, the installed package, expected content paths,
free space and staged code digest. It skips content and code transfers when matching
staging already exists, and never starts the game:

```sh
bash scripts/install-game.sh
```

Assets are at `/sdcard/Android/data/dev.xcaptain.muguet/files/game/`. Code stages at
`.../files/game-code/game-android.cwasm`. On ROMs where `run-as` works, the installer
copies the code into private `files/game-code/`; on the Realme X7 Max, `run-as` is
denied by SELinux. GameActivity therefore checks its private copy on a background
worker, copies staged code through a `.part` file and atomic rename if needed, and
starts native loading only after the private copy is ready. Saves are under private
`files/saves/`, and runtime cache is under private `cache/game/`. No root, storage
permission, SELinux change, or outside server is needed.

## Status and logs

The activity displays the native phase (`accepted`, `module_ready`, `running`,
`present_requested`, or `failed`), error, presentation count, validation-error count,
and heap bytes. It polls without blocking UI and exports each JSON snapshot to its
own external files directory. The isolated process also redirects Rust stderr to
private `files/game-host.log` and exports its last 2 MiB. Read both on this ROM with:

```sh
adb pull /sdcard/Android/data/dev.xcaptain.muguet/files/game-status.json .
adb pull /sdcard/Android/data/dev.xcaptain.muguet/files/game-host.log .
adb logcat -s MuguetGame:I
```

On a ROM permitting `run-as`, the full private log can additionally be read with
`adb exec-out run-as dev.xcaptain.muguet cat files/game-host.log > game-host-full.log`.
The exported log is a bounded tail; the private log persists for detailed diagnosis.

Directional controls, Enter, Escape, action (`E`), jump (Space), touch look drag,
and corresponding keyboard keys are available. Focus loss releases all held inputs.
