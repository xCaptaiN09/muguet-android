# Muguet Android

An experimental Android port project based on the approach used by
[Muguet](https://github.com/c22dev/muguet): run the GTA V browser build locally
with a WebAssembly runtime and a native graphics backend.

**Status:** the Android game host renders GTA V's real startup screen locally on
the Realme X7 Max running Android 16/API 36. Android then terminates the session
for low memory. An interactive world and sustained gameplay are not yet verified.
The earlier shared-memory/Vulkan diagnostic foundation remains available.

## End goal

Run GTA V locally on an Android smartphone using the browser-build game code
and assets, with Muguet as the starting point for the native host. The initial
device is the Realme X7 Max. The intended result is interactive gameplay with
touch controls, graphics, and sound, with single-player gameplay available
offline after the game files are installed.

The compatibility APK below is a development milestone toward that result.

## First milestone: verified

Build a compatibility APK that runs a small threaded WebAssembly sample and
renders through Vulkan using `wgpu`. Record GPU capabilities, runtime results,
and memory use on the target phone before integrating the game.

This milestone does not require GTA V game files.

On-device results: guest atomics, bounded memory growth, cross-thread
wait/notify, Vulkan presentation, and JSON report export passed. Rotation and
background/relaunch checks invalidate the old surface result and allow a fresh
successful run. The GPU reports **no BC texture support**, so game integration
will need the texture-decoding fallback.

See [device validation](docs/DEVICE_VALIDATION.md) for evidence and limits, and
[game-host instructions](docs/GAME_HOST.md) for the experimental game build.

## Build and test

On a Linux x86-64 host with the documented Android SDK/NDK and JDK 21 installed:

```bash
bash scripts/setup-rust.sh
bash scripts/build-native.sh
bash scripts/build-apk.sh
```

The current debug APK is written to `artifacts/muguet-android-game-debug.apk`.
Building a game-capable APK requires the trusted Android compiled-code digest
described in the game-host instructions. Game code and assets are installed
separately. The retained foundation APK is 13.28 MiB.
See [testing instructions](docs/TESTING.md) for tool versions and phone checks.

## Initial target device

- Realme X7 Max
- MediaTek Dimensity 1200 / Mali-G77 MC9
- 8 GB RAM
- Android 16, as reported by the device owner

Startup rendering is verified on this device. Gameplay and sustained performance
remain unverified; the first sessions ended with Android's `LOW_MEMORY` reason.

## Stack

- Rust for the native host and WebAssembly runtime integration
- Wasmtime targeting ARM64 Android
- `wgpu` with the Vulkan backend
- Android SDK/NDK for application integration and APK packaging

The game host uses Android-targeted precompiled Wasmtime code, Vulkan, CPAL/AAudio,
native local file reads, and an isolated Android game process.

## Game files

No game binaries, shaders, or assets are included in this repository. The game
host uses the browser build's `game.wasm`, `shaders/`, and `data/` files. A standard
PC installation is not the input format Muguet expects.

Keep any local game files under `game-files/`, which is ignored by Git.

## Roadmap and upstream

See [the roadmap](docs/ROADMAP.md) for milestones and acceptance checks.
See [the upstream porting audit](docs/PORTING_AUDIT.md) for the source review
and the role of the browser snapshot repository.
See [asset delivery](docs/ASSET_DELIVERY.md) for local-file and progressive
download directions.

This is an unofficial community project and is not affiliated with Rockstar
Games, Take-Two, or Muguet's author. Imported Muguet host code retains its
GPL-3.0-or-later license and [upstream attribution](third_party/muguet/UPSTREAM.md).
