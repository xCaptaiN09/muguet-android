# Muguet Android

An experimental Android port project based on the approach used by
[Muguet](https://github.com/c22dev/muguet): run the GTA V browser build locally
with a WebAssembly runtime and a native graphics backend.

**Status:** project initialized. There is no runnable Android port or APK yet.

## End goal

Run GTA V locally on an Android smartphone using the browser-build game code
and assets, with Muguet as the starting point for the native host. The initial
device is the Realme X7 Max. The intended result is interactive gameplay with
touch controls, graphics, and sound, with single-player gameplay available
offline after the game files are installed.

The compatibility APK below is a development milestone toward that result.

## First milestone

Build a compatibility APK that runs a small threaded WebAssembly sample and
renders through Vulkan using `wgpu`. Record GPU capabilities, runtime results,
and memory use on the target phone before integrating the game.

This milestone does not require GTA V game files.

## Initial target device

- Realme X7 Max
- MediaTek Dimensity 1200 / Mali-G77 MC9
- 8 GB RAM
- Android 16, as reported by the device owner

Compatibility and game performance on this device are unverified.

## Proposed stack

- Rust for the native host and WebAssembly runtime integration
- Wasmtime targeting ARM64 Android
- `wgpu` with the Vulkan backend
- Android SDK/NDK for application integration and APK packaging

The compatibility milestone will determine which runtime and graphics changes
are needed before the full port.

## Game files

No game binaries, shaders, or assets are included in this repository. A later
game integration will need the browser build's `game.wasm`, `shaders/`, and
`data/` files. A standard PC installation is not the input format Muguet expects.

Keep any local game files under `game-files/`, which is ignored by Git.

## Roadmap and upstream

See [the roadmap](docs/ROADMAP.md) for milestones and acceptance checks.
See [the upstream porting audit](docs/PORTING_AUDIT.md) for the source review
and the role of the browser snapshot repository.

This is an unofficial community project and is not affiliated with Rockstar
Games, Take-Two, or Muguet's author. No upstream source code has been imported
at initialization. Muguet is licensed under GPL-3.0; retain its license and
attribution when importing its code.
