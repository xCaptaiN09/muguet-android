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

The development installer checks ADB, the installed package, free space and staged
code digest. It writes a content identity receipt only after a successful transfer,
complete path/size comparison, and directory permission normalization. A matching
receipt and a fresh path/size comparison skip the bulk asset transfer. On every
install, including receipt reuse and adoption, it adds read/traverse permission to
directories under external `game/data/` and `game/b/` so the app can read assets
created by ADB. If normalization fails, installation exits before reporting ready.
It never starts the game:

```sh
bash scripts/install-game.sh
```

The existing 12,156-file external staging tree predates receipts. Adopt it deliberately
with `bash scripts/install-game.sh --adopt-existing-assets`: this compares every relative
path and size to the PC prepared tree without hashing 15 GB of phone content. An
interrupted or changed tree fails adoption. Use `--repair-assets` to resume/repair
with ADB sync and write a receipt only after the path/size map matches. The receipt
is stored as `.muguet-content-receipt` under the app's external `game/` directory
and is excluded from that comparison.

Assets are at `/sdcard/Android/data/dev.xcaptain.muguet/files/game/`. Code stages at
`.../files/game-code/game-android.cwasm`. On ROMs where `run-as` works, the installer
copies the code into private `files/game-code/`; on the Realme X7 Max, `run-as` is
denied by SELinux. GameActivity therefore checks its private copy on a background
worker, copies staged code through a `.part` file and atomic rename if needed, and
starts native loading only after the private copy is ready. Saves are under private
`files/saves/`, and runtime cache is under private `cache/game/`. No root, storage
permission, SELinux change, or outside server is needed.

## Status and logs

The activity shows readable loading/failure and hides technical status once presentation
begins. Its small Info toggle reveals phase, presentation count, validation-error count
and heap bytes. Status JSON also includes `inVehicle` and the requested graphics profile.
It polls without blocking UI and exports each JSON snapshot to its own external files directory. The isolated process also redirects Rust stderr to
private `files/game-host.log` and exports its last 2 MiB. Read both on this ROM with:

```sh
adb pull /sdcard/Android/data/dev.xcaptain.muguet/files/game-status.json .
adb pull /sdcard/Android/data/dev.xcaptain.muguet/files/game-host.log .
adb logcat -s MuguetGame:I
```

On a ROM permitting `run-as`, the full private log can additionally be read with
`adb exec-out run-as dev.xcaptain.muguet cat files/game-host.log > game-host-full.log`.
The exported log is a bounded tail; the private log persists for detailed diagnosis.

Native Canvas mobile controls replace the provisional row. A floating left stick,
independent camera drag, contextual fire/gas, aim/brake, jump/handbrake, enter/exit,
reload/horn and small menu controls support simultaneous pointers. Hardware keyboard
and Android Back remain available. Focus loss, cancellation, context/menu changes,
pause, stop, surface loss and failed sessions release controls immediately. See
[MOBILE_CONTROLS.md](MOBILE_CONTROLS.md) for mappings, accessibility, wheel adaptation
and requested conservative graphics flags in version 0.3.0-mobile.

Earlier device runs proved local legal/startup rendering, then Android reported
LOW_MEMORY at 2.9 GB and 2.2 GB RSS. No playable world is verified. The graphics retry
profile is requested and logged; it does not establish that memory pressure is fixed.
The existing compiled artifact is reused with its embedded trusted digest; no AOT,
heap minimum or original/hardlinked asset/settings changes are part of this retry.
