# Android Game Host Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Integrate Muguet's real game host into Android and attempt the first game boot on the X7 Max using the verified local browser build.

**Architecture:** A reusable Rust game-host crate replaces Apple integrations with Vulkan and Android-compatible platform functions. A separate Android game process owns one session and its surface; the existing diagnostic activity remains available. Compile Android-native game code on the PC, pin its checksum in the APK, and install assets/code over USB.

**Tech Stack:** Rust 1.99.0, Wasmtime 49.0.2, wgpu 30.0.1 Vulkan, Java Android Views, JNI, AGP 8.13.2, API 29 minimum/API 36 target, NDK 28.2.13676358.

**Spec:** `docs/superpowers/specs/2026-10-12-android-game-host-design.md`

## Global Constraints

- Game execution is local; no streaming or asset server is part of this milestone.
- Preserve the diagnostic API and original retained archives; game assets and compiled code stay outside Git.
- Source pin: `ddcc54f44ae2a21b8c6e43ec1c925b55b352ffbd`; preserve GPL-3.0-or-later licensing and attribution.
- Keep Rust 1.99.0, Wasmtime 49.0.2 and wgpu 30.0.1; Android ARM64/API 29 minimum/API 36 target.
- Use a 5 GiB maximum heap, 5 GiB reservation, 64 KiB guard, non-moving shared memory, memory64 and threads.
- Precompile and load with matching generic `aarch64-linux-android` configuration.
- Verify compiled code against an APK-embedded SHA-256 before unsafe deserialization; use app-private storage.
- `SurfaceHandoff` keeps the acquired native window alive until after its Vulkan surface is dropped.
- Use a separate unexported `GameActivity` in process `:game`; backgrounding ends this first experimental session.
- Initial launch: 960x432 landscape Sandbox, 30 FPS, BC CPU fallback as needed; report real execution progress and failures.
- No root, broad storage permission, or phone system-setting changes are needed.

### Task 1: Reusable host and Android code compiler

**Files:**
- Create `game-host/Cargo.toml`, `game-host/src/lib.rs`, `game-host/src/platform.rs`, `game-host/src/session.rs`, `game-host/src/code.rs`, `game-host/src/bin/precompile-android.rs`.
- Import/adapt required upstream modules under `game-host/src/` (host, fs, io, input, audio, gpu, patch, proto, mp/server if required by the patch hooks); preserve source headers and minimap shader.
- Create `LICENSE`, `third_party/muguet/UPSTREAM.md` and a source archive/provenance record without game content. Retain the vendor wgpu-core patch only if needed; document its actual changes.
- Modify root `Cargo.toml` and `Cargo.lock`; do not modify Android UI or diagnostic JNI.
- Test `game-host/tests/host_contract.rs` plus focused module tests.

**Interfaces:**
- `LaunchConfig { game_root: PathBuf, code_path: PathBuf, expected_code_sha256: String, saves_dir: PathBuf, cache_dir: PathBuf, width: u32, height: u32 }`.
- `SurfaceHandoff { instance: wgpu::Instance, surface: wgpu::Surface<'static>, owner: Arc<dyn Send + Sync> }`.
- `pub fn launch(config: LaunchConfig, surface: SurfaceHandoff) -> anyhow::Result<()>` accepts at most one session per process and returns promptly; runtime work runs on owned threads.
- `pub fn status() -> serde_json::Value` includes `phase`, nullable `error`, `presents`, `gpuErrors`, `heapBytes`; no gameplay certification.
- `pub fn suspend(paused: bool)`, `pub fn key(code: &str, pressed: bool)`, `pub fn mouse_delta(dx: f64, dy: f64)`.
- CLI: `precompile-android <game.wasm> <output.cwasm>` writes the compiled artifact and `<output.cwasm>.sha256` containing the hex digest plus newline, and prints machine-readable output including digest/configuration.

- [x] Write failing boundary tests before new behavior. Include missing content, ambiguous build selection, invalid dimensions, digest mismatch without native-code deserialization, and configured heap cap. Use small real Wasm fixtures where execution is needed; retain RED evidence in the report.

```rust
#[test]
fn rejects_untrusted_compiled_code_before_deserialization() {
    let result = muguet_engine::code::verify_bytes(b"not native code", &"0".repeat(64));
    assert!(result.unwrap_err().to_string().contains("SHA-256"));
}
```

- [x] Import the pinned source and implement platform adaptation. Replace desktop window ownership with the opaque surface owner, select Vulkan, gate Apple scheduling/memory APIs, use target-correct libc resource limits, and use an Android audio backend. Threaded GPU initialization errors/traps update session failure; do not leave a falsely healthy drain loop. Guest process exit becomes a reported engine exit rather than terminating the Android launcher.
- [x] Implement identical Android compiler/runtime configuration and patched-module compilation, content discovery, typed launch/session gates and checksum-verified native-code loading. Preserve upstream fixes; multiplayer stays disabled for this milestone. Do not invent missing engine imports.
- [x] Run `source scripts/env.sh; cargo test -p muguet-engine` and a locked `cargo check -p muguet-engine --target aarch64-linux-android`. Document any Android audio context requirement for Task 2. Run formatter and covering clippy checks; report imported-source warnings separately from newly introduced errors.
- [x] Compile the actual verified game module using the new CLI to `~/.local/share/muguet-android/compiled/full-game-v1/game-android.cwasm`; retain its digest and timing. Use the shared project toolchain. No phone deployment in this task.
- [x] Self-review and commit. Write the complete report with commands, RED/GREEN evidence, cross-target result, actual compile result/digest, and concerns.

### Task 2: Android launch path, native bridge and development installer

**Files:**
- Create `native/src/game_android.rs`, `native/build.rs`, Android `GameActivity.java`, `GameSessionState.java` and its JVM tests.
- Modify native Cargo/lib wiring, `NativeBridge.java`, `MainActivity.java`, manifest, strings, app version and `scripts/build-native.sh`.
- Create `scripts/precompile-game.sh`, `scripts/prepare-game.py`, `scripts/install-game.sh`, `docs/GAME_HOST.md` and meaningful preparation tests.

**Interfaces:**
- Consume Task 1's exact public API and artifact format; inspect its report for any ruled interface adjustments.
- Native bridge methods: `startGame(Surface surface, String gameRoot, String codePath, String savesDir, String cacheDir, int width, int height): String`, `gameStatus(): String`, `pauseGame(boolean paused): void`, `gameKey(String code, boolean pressed): void`, `gameLook(float dx, float dy): void`.
- `MUGUET_ANDROID_CODE_SHA256` is a build-time environment variable embedded with `option_env!` in native Rust; invalid/missing pin prevents game launch without breaking diagnostics. Never take the expected digest from an Intent, adjacent phone file, or editable UI.
- Phone assets: `getExternalFilesDir(null)/game`; native code: `getFilesDir()/game-code/game-android.cwasm`; saves: `getFilesDir()/saves`; cache: `getCacheDir()/game`.

- [x] Write meaningful failing tests for session-state gating/control release and preparation filters before implementing them. For preparation, test case-insensitive skipped components and ensure manifest/DLC edits cannot modify hardlinked originals.
- [x] Wire JNI using owned ANativeWindow references and guarded errors/panics. Initialize audio/JVM context if Task 1's backend requires it. Calls return promptly; no JNI local references or JNIEnv cross threads. Poll status in GameActivity and display the real phase/error.
- [x] Add launcher entry and a landscape SurfaceView game activity in `:game`. Use fixed 960x432 render buffer, basic directional/Enter/Escape/action/jump controls and look dragging. Release held inputs on focus loss; pause first, then terminate only the game process on stop/destroy. Document the experimental session restart behavior.
- [x] Add reproducible preparation/install scripts. Default PC content source is the verified extracted build and output is a separate development tree. Keep all originals untouched. The installer checks ADB connectivity, app package, expected directories/free space, and uses app-private run-as where available, or app-owned external staging followed by app-side atomic private copy when the ROM denies run-as. Verify against the baked-in pin before native-code deserialization; do not weaken SELinux or require root. Do not deploy or overwrite unrelated data as part of a build.
- [x] Build with the actual artifact's digest pinned. Run Python preparation tests, Rust host/Android checks, Java tests/lint, and assemble APK; verify signature and 16 KiB ZIP/ELF alignment. Document commands and actual results.
- [x] Self-review and commit, reporting exact APK/artifact paths, version, SHA-256, and any known limitations. Do not claim real-game success without device evidence.

### Task 3: Real content installation and device boot

**Files:**
- Modify only affected implementation files for evidence-driven fixes.
- Update `docs/GAME_HOST.md`, `docs/ROADMAP.md`, `docs/PORTING_AUDIT.md` and `docs/DEVICE_VALIDATION.md` with exact observed results.
- Preserve ignored `artifacts/game-boot/` logs, screenshots and native status reports.

**Interfaces:**
- Use Task 2's installer/launch activity and Task 1's status; the real phone is already authorized for USB development.
- Preserve the original diagnostic APK at `/home/captain/muguet-android/artifacts/muguet-android-debug.apk`.

- [ ] Install the game-host APK without clearing existing app data. Prepare/transfer selected content and the pinned code, checking phone free space first. Keep original PC archives.
- [ ] Launch the real game and capture logs/status/screenshots. Use Android CLI skills for visual checks; inspect every captured screenshot immediately before subsequent device actions. Ask for unlocking only if the lockscreen blocks required testing.
- [ ] Diagnose concrete runtime/shader/texture/memory failures with a reproduction before changing code. Delegate code fixes for review, rerun their covering checks, rebuild/reinstall and retry. Do not replace missing behavior with a synthetic success.
- [ ] Verify basic visible game output/input/audio if boot reaches a scene, then verify background closes only the game process and launcher diagnostics still run. Otherwise record the exact trap/failure and retain its logs.
- [ ] Update completion evidence truthfully, commit, run whole-branch review and address findings. Push the private feature branch; no main merge or public release is requested.
