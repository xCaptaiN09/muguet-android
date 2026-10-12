# Android Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Build an installable ARM64 Android app that tests the runtime and Vulkan path needed for a Muguet-based GTA V port.

**Architecture:** A Java Activity owns an Android SurfaceView and calls a Rust JNI library on a worker thread. The Rust crate provides real Wasmtime probes and Vulkan rendering. JSON results are shown and exported by the app; the same native library will host subsequent game integration.

**Tech Stack:** Java, Android SDK/NDK, Gradle, Rust, Wasmtime, wgpu/Vulkan, JNI.

**Spec:** `docs/superpowers/specs/2026-10-12-android-foundation-design.md`

## Global Constraints

- ARM64 Android only, minimum API 29, compile/target API 36.
- Android Gradle Plugin 8.13.2, Gradle 8.13, JDK 21.
- NDK 28.2.13676358; native library load segments aligned to at least 16 KiB.
- Rust 1.99.0, Wasmtime 49, wgpu 30.0.1, JNI 0.21.
- No root requirement, internet permission, game files, hosting deployment, or claims of GTA gameplay in this build.
- Runtime probe memory is bounded to two 64 KiB pages; every cross-thread wait has a timeout.
- JNI returns JSON with `status`; failures cannot unwind across JNI.

## Setup and ownership

The controller owns `Cargo.toml`, `native/Cargo.toml`, `scripts/`, documentation,
toolchain installation, and integrated build verification. Work happens on
`feat/android-foundation` in the existing user-selected project directory.
Preserve the upstream audit already committed on this branch.

### Task 1: Native Wasmtime runtime probes

**Files:** Create `native/src/lib.rs` initially exporting `runtime`,
`native/src/runtime.rs`, `native/tests/runtime_probe.rs`,
`native/src/bin/check_runtime.rs`.

**Interfaces:** Produce `pub fn run_runtime_probe() -> serde_json::Value`.
Consume the workspace manifest; do not change Cargo dependencies without
coordinating with the controller. JNI consumes the returned object unchanged.

- [ ] Write a behavior test expecting a passed report containing actual guest atomic read/write results, bounded memory growth, and cross-thread wait/notify results. A wrong expected guest value, missing notification, or successful out-of-bound growth must fail the test.
- [ ] Run `cargo test -p muguet-android-native --test runtime_probe`; record the expected initial failure before implementing the probe.
- [ ] Implement the shared memory64 guest using the module form below, then create per-thread Wasmtime stores over the same shared memory. Match the Wasmtime version's typed signatures and use a bounded notify retry loop to avoid a lost-wakeup race.

```wat
(module
  (import "env" "memory" (memory i64 1 2 shared))
  (func (export "write") (param i64 i32)
    local.get 0 local.get 1 i32.atomic.store)
  (func (export "read") (param i64) (result i32)
    local.get 0 i32.atomic.load)
  (func (export "wait") (param i64 i32 i64) (result i32)
    local.get 0 local.get 1 local.get 2 memory.atomic.wait32)
  (func (export "notify") (param i64 i32) (result i32)
    local.get 0 local.get 1 memory.atomic.notify))
```

- [ ] Return `status`, `checks`, and elapsed time, including exact observed values in each check's `details`. Convert runtime errors into failed JSON, with no fabricated results.
- [ ] Add a host CLI that prints the report and exits nonzero when `status` is failed. Run the behavior test and CLI, format the files, and record commands and outputs in the task report.

### Task 2: Android application and report export

**Files:** Own all files under `android/`, including wrapper, settings/build
files, manifest, Java app classes, resources, and Java tests.

**Interfaces:** Call `NativeBridge.runRuntimeChecks()` and
`NativeBridge.renderSurface(Surface, int, int)` from a serial worker. Load
`muguet_android`. Gradle's native build task calls
`bash ../scripts/build-native.sh <app-build-dir>/generated/rustJni` from the
`android/` directory and packages that directory as `jniLibs`.

- [ ] Write independent Java tests for report assembly: a failed native result must keep the combined result failed; malformed native JSON must not appear successful; pending/skipped graphics cannot certify compatibility.
- [ ] Observe the initial failure, then implement report assembly using `org.json` with an explicit real JVM dependency for unit tests. Do not mock the JSON library.
- [ ] Create a framework Activity with SurfaceView, Run checks, result text, and Export report controls. Use worker execution, disable repeated concurrent runs, handle surface recreation and Activity destruction, and apply system-bar insets.
- [ ] Export actual JSON through `ACTION_CREATE_DOCUMENT` without storage permissions. Display the exact early-build scope and never show a working Play GTA button.
- [ ] Pin the build versions from Global Constraints; package ARM64 only and add an Android 16-compatible debug manifest. Generate a Gradle 8.13 wrapper using a verified distribution checksum.
- [ ] Run `:app:testDebugUnitTest` and Java compilation/Android lint where available. Record results; the controller performs the final APK assembly after native integration.

### Task 3: Vulkan renderer, JNI bridge, and native integration

**Files:** Modify `native/src/lib.rs`; create `native/src/gpu.rs`,
`native/src/android.rs`, `native/src/bin/check_gpu.rs`, and focused tests for
validation/error paths.

**Interfaces:** Export `runtime` and `gpu` modules. Produce
`gpu::probe_vulkan() -> serde_json::Value`. JNI symbols are
`Java_dev_xcaptain_muguet_NativeBridge_runRuntimeChecks` and
`Java_dev_xcaptain_muguet_NativeBridge_renderSurface` with the spec's signatures.

- [ ] Test invalid render dimensions and the error-to-JSON boundary first, then implement validation. Reject zero, negative, and oversized dimensions before resource creation.
- [ ] Implement Vulkan adapter discovery and collect real capabilities without requiring a display surface on the host. A host probe must not claim that a frame was presented.
- [ ] Implement the Android surface handoff using `ANativeWindow_fromSurface` and a matching release guard. Keep the acquired window alive until wgpu surface/device work is finished, including failed paths.
- [ ] Configure the surface, submit a clear-color render pass, and present once. Return accurate adapter details, features, limits, BC texture support, and presentation result. Handle errors explicitly.
- [ ] Implement both JNI methods with panic containment and Java-string error handling. Native failures must produce failed JSON or a Java exception, never successful placeholder text.
- [ ] Run relevant native tests and the host GPU CLI where hardware is available. Cross-compile the Android library and record verification in the task report.

## Integrated acceptance

- [ ] Review all three task diffs for spec compliance and correctness.
- [ ] Run `cargo fmt --all --check`, native behavior tests, Java report tests, Android lint, and `bash scripts/build-apk.sh` with the project environment selected.
- [ ] Inspect APK contents, package/API metadata, JNI symbols, and ELF segment alignment; record the APK size and SHA-256.
- [ ] If ADB has a connected device, install and run the app and collect its report. Otherwise provide `artifacts/muguet-android-debug.apk` and explicit installation/report instructions, with phone execution marked unverified.
- [ ] Update README and roadmap with the actual verified state. Do not merge or publish the feature branch without existing authorization or a concrete user review.
