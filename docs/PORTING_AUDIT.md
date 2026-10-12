# Upstream porting audit

Reviewed on 2026-10-12. This is a static code and manifest review. Neither the
game nor an Android build has been executed.

## Sources reviewed

- [c22dev/muguet](https://github.com/c22dev/muguet), commit
  `a48e9a71e13c7ae4b161f72ea0bff868b85588c7`.
- [alihammoud21/playgta5](https://github.com/alihammoud21/playgta5), commit
  `4a0d596d9b030be20811f0d5c8882f1e2ce05c07`.

Both repositories were checked out separately for inspection. No upstream code,
game binaries, game assets, or screenshots have been copied into this project.
No release archives were downloaded and no upstream scripts were executed.

## What the supplied screenshots show

The launcher visibly credits c22dev and shows Sandbox mode, 50% resolution,
and a 60 FPS frame limit. Other frames display gameplay and FPS/network
overlays. This is consistent with the Muguet project being the intended
reference. The images alone do not establish sustained performance, hardware
identity, or whether execution is local. The upstream implementation provides
the technical evidence for Muguet's local execution approach.

## Role of each repository

Muguet supplies the native host: Wasmtime integration, engine imports, threaded
data reads, graphics command processing, input, audio, and Apple app integration.
This is the primary implementation to adapt for Android.

PlayGTA5 supplies browser-snapshot installation, hosting, verification, and
diagnostic helpers. Its ordinary Git checkout contains about 3.3 MB of files.
It does not contain the original game-engine source or the game binary; its
inventory and release manifest describe the separately distributed snapshot.

The published inventory describes 12,482 mirror files totaling 21,158,604,965
bytes (about 21.2 GB / 19.7 GiB). Summing the inventory entries reproduced both
totals. This checks the inventory's internal consistency, not the downloadable
files themselves.

Relevant inventory entries are:

| Entry under `mirror/playgta5.com/` | Published size |
| --- | ---: |
| `b/8b0b5899ed/game.wasm` | 63,201,802 bytes |
| `b/8b0b5899ed/shaders/index.json` | 685,955 bytes |
| `data/manifest.json` | 393,014 bytes |

These paths match the layout checked by Muguet's `src/import.rs`. That makes
this snapshot a candidate input for a future game test; actual runtime and
shader compatibility remain unverified.

## Confirmed Android porting work

| Area | Evidence in Muguet | Android work |
| --- | --- | --- |
| Graphics backend | `src/app.rs` and `src/gpu.rs` select `Backends::METAL` | Select Vulkan, create an Android surface, and validate renderer behavior on Mali. |
| Apple dependencies | `Cargo.toml` declares `objc2` and Apple controller crates unconditionally | Gate Apple dependencies and provide Android implementations. |
| Native entry point | `src/main.rs` uses a desktop-style `main`; Apple launchers live separately | Add an Android native entry point, packaging, and lifecycle handling. |
| Thread scheduling | `src/main.rs` calls `pthread_set_qos_class_self_np` | Replace or gate Apple thread-priority behavior. |
| Memory reporting | `src/main.rs` calls `proc_pid_rusage` | Report Android process memory using an appropriate platform implementation. |
| Resource limits | `src/main.rs` defines platform constants and a resource-limit structure manually | Use target-correct definitions and handle Android limits. |
| Runtime configuration | iOS has a configurable heap cap; the other path reserves 17 GiB plus guard space | Define Android-specific memory settings and measure resident use. Virtual reservation is not physical RAM consumption. |
| Runtime compilation | iOS precompilation targets `aarch64-apple-ios` | Compile for Android or use Android-native JIT and caching after validation. iOS artifacts cannot be assumed portable. |
| Controls and launcher | `src/pad.rs`, `src/touch.rs`, and Swift launchers use Apple APIs | Implement Android touch/controller input and launch settings. |
| Storage and audio | `src/io.rs` uses positional Unix file reads; `src/audio.rs` uses CPAL's default host | Provide Android storage paths/import and verify the selected audio backend. |

The renderer already has CPU decoding for BC textures when the GPU does not
support them, plus a fallback for filtering 32-bit float textures. These are
useful starting points, but their correctness and cost on the target phone
must be measured.

PlayGTA5's `serve_local.py` implements byte ranges and a batched data endpoint.
Muguet already replaces browser HTTP reads with a native file-reading worker,
so the Android host should retain native data access rather than require a
Python server on the phone.

## Recommended next implementation

Build the first Android app around the intended Rust host, with a Vulkan
surface and a bounded synthetic Wasm test covering memory64, shared memory,
and atomic wait/notify. Keep these diagnostics in the same app foundation that
will later launch the game.

Then adapt Muguet's host and renderer, import a separately supplied browser
snapshot, and aim for a repeatable first interactive scene at low resolution.
After boot, validate movement, touch controls, sound, saves, and extended
offline sessions. Frame-rate claims require measurements on the Realme.

## Pinned code references

- [Muguet runtime and platform calls](https://github.com/c22dev/muguet/blob/a48e9a71e13c7ae4b161f72ea0bff868b85588c7/src/main.rs)
- [Muguet graphics renderer](https://github.com/c22dev/muguet/blob/a48e9a71e13c7ae4b161f72ea0bff868b85588c7/src/gpu.rs)
- [Muguet importer](https://github.com/c22dev/muguet/blob/a48e9a71e13c7ae4b161f72ea0bff868b85588c7/src/import.rs)
- [Muguet native data reader](https://github.com/c22dev/muguet/blob/a48e9a71e13c7ae4b161f72ea0bff868b85588c7/src/io.rs)
- [PlayGTA5 file inventory](https://github.com/alihammoud21/playgta5/blob/4a0d596d9b030be20811f0d5c8882f1e2ce05c07/file-inventory.json)
- [PlayGTA5 browser diagnostics](https://github.com/alihammoud21/playgta5/blob/4a0d596d9b030be20811f0d5c8882f1e2ce05c07/support/check.js)
