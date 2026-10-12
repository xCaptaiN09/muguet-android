# Android game host and first boot

## Approved direction

The user approved starting actual game integration after downloading and
verifying the browser snapshot. The game executes locally on the Realme X7 Max
(ARM64, Android 16, 8 GB RAM, Mali-G77 MC9). No streaming or asset server is part
of this milestone. Retain the diagnostic foundation and all original archives.

This is a new native subsystem. Reuse Muguet's engine host and renderer rather
than reimplement its engine imports. A browser/WebView wrapper would require a
different graphics and threading path. A Windows emulator would use different
game binaries and does not implement the approved Muguet approach.

## Source and content

Import the required host sources from c22dev/muguet commit
`ddcc54f44ae2a21b8c6e43ec1c925b55b352ffbd`, including its recent movie-preload,
scenario-collision, pipeline-wait, BC-decoding and minimap fixes. Preserve
GPL-3.0-or-later licensing, original source headers, attribution, and provenance.
Keep adaptation changes separate from the preserved upstream source record.

The independently retained `full-game-v1` snapshot contains 14 SHA-256-verified
archives and 12,482 SHA-256-verified mirror files. Development content is under
`~/.local/share/muguet-android/game-files/full-game-v1/mirror/playgta5.com/`.
Game code is `b/8b0b5899ed/game.wasm`. Game assets and compiled code remain outside
Git. Prepare a separate Android content tree, excluding path components named
`env_test` or `verityradio` case-insensitively, and tidy the copied manifest and
DLC list as Muguet's importer does. Never modify hardlinked original content.

## Architecture

Add a `muguet-engine` Rust workspace crate containing the reusable host,
filesystem, I/O, input, audio, patching and Vulkan renderer. Apple dependencies,
desktop window creation, Apple scheduling, Apple process metrics and hardcoded
Apple resource-limit definitions cannot enter the Android build. Preserve the
existing `muguet-android-native` diagnostic API.

Compile patched game Wasm to `aarch64-linux-android` on the PC using the same
Wasmtime 49.0.2 configuration the phone will use. Use a 5 GiB maximum heap,
5 GiB reservation, 64 KiB guard, non-moving shared memory, memory64 and threads.
Use a generic ARM64 target consistently on both sides; do not assume optional
CPU instructions. Compilation outputs a native-code artifact and its SHA-256.
The APK embeds that digest at build time. Reject a missing/mismatched digest
before unsafe Wasmtime deserialization. The artifact installed on the phone
must be in app-private storage. No downloaded arbitrary native-code artifact
may be deserialized on the authority of a user-writable adjacent checksum file.

The engine exposes `LaunchConfig`, `SurfaceHandoff`, `launch`, `status`,
`suspend`, `key` and `mouse_delta`. `launch` accepts one session per process,
starts work off the caller thread, and reports errors through `status`.
`LaunchConfig` contains game_root, code_path, expected_code_sha256, saves_dir,
cache_dir (PathBuf except the String digest), width and height (u32).
`SurfaceHandoff` owns a Vulkan instance, a Surface<'static> and an opaque
Arc<dyn Send + Sync> native-window owner. The owner outlives the surface on all
success, failure and panic paths. Engine status includes phase, error, presents,
GPU validation-error count, and current heap bytes. Presentation counters never
certify gameplay or visible scanout.

Use a separate, unexported Android `GameActivity` in process `:game`, preserving
the existing launcher and diagnostic screen in the main process. The activity
creates a SurfaceView, calls JNI asynchronously, and polls status without
blocking the UI. The initial target is a 960x432 landscape Sandbox session at
30 FPS, with BC CPU decoding on this Mali device. The real game's behavior
determines whether those settings need adjustment. Include basic directional,
Enter, Escape, action/jump controls and a look-drag region. Release inputs on
focus loss. Sound should use an Android-supported output backend.

For this first boot milestone, backgrounding or destroying GameActivity ends
the isolated game process, and reopening starts a fresh session. Suspend and
release inputs before ending it. This bounds ownership of global engine/GPU
threads and avoids reusing a destroyed Android surface. Full background/resume
with an ongoing session is a later playability task. Ending this process must
not terminate the launcher process or change phone system settings.

## Storage and installation

Assets live in the app's own external files directory at `game/`; compiled
code, saves and runtime cache live in app-private files/cache directories.
No root or broad storage permission is needed. For this development milestone,
install verified prepared content by USB/ADB, and install the locally compiled
code using the debug app's run-as facility when available. On this ROM run-as
is denied by SELinux; stage code in the app's own external files directory, then
have the app copy it atomically into private storage before trusted-checksum
validation/loading. Never change SELinux or require root for this fallback.
Do not transfer until paths and free space have been checked. Do not overwrite unrelated phone files. Keep all
source archives on the PC; there is no hosted backup yet.

## Failure reporting and verification

Reject absent content/build selection, invalid dimensions and untrusted native
code with specific errors. Distinguish accepted launch, module ready, engine
running, GPU presentation requested, and failed. Engine traps and GPU errors
must remain visible in status/logs instead of becoming a false success.

Test meaningful boundaries: content discovery, heap configuration, digest
rejection before deserialization, lifecycle/session gating and control release.
Run host tests, Android cross-build, Java tests and lint, APK signature/alignment
checks, then install on the actual phone. Validate the actual downloaded module
and attempt boot with collected logs and screenshots. Diagnose concrete failures
and repeat after fixes. Report precisely how far execution reached. No FPS,
playability or memory-safety certification follows from the foundation probes.

## Completion

This implementation milestone delivers an Android-capable real game host,
reproducible code/content preparation, an APK with the launch path, and an
evidenced real-game boot attempt on the X7 Max. Continue through fixable boot
issues toward a visible interactive scene. If a specific unresolved issue
prevents that scene, retain the reproducible failure, report the exact limit,
and keep the broader gameplay objective open.
