# Android port roadmap

## 0. Repository setup

- Initialize the local project and Git history.
- Create the GitHub repository under `xCaptaiN09`.
- Document the target device and keep game files out of version control.

## 1. Compatibility APK

Build a small ARM64 Android application before importing the game host.

- Run a synthetic Wasm sample with shared memory and atomic wait/notify.
- Check memory64 support with a bounded test allocation.
- Create an Android rendering surface and present a test frame using `wgpu`
  with Vulkan.
- Report GPU identity, feature flags, relevant limits, and BC texture support.
- Exercise pause, resume, and surface recreation.
- Export a diagnostic report without requiring root access.

Acceptance: the APK installs and these checks produce a reproducible report on
the Realme X7 Max. Report failures explicitly; a successful test frame alone
does not establish game compatibility.

## 2. Android host integration

- Review and import the necessary Muguet code with its license and attribution.
- Isolate Apple-specific dependencies and replace Metal backend selection.
- Integrate Android input, audio, file import, logging, and app lifecycle.
- Validate shaders and texture fallbacks against the device's Vulkan driver.
- Choose Android-targeted runtime compilation and caching after the runtime
  probe; do not assume an iOS precompiled artifact is portable to Android.

Acceptance: the native host runs on Android and its platform integrations work
with synthetic fixtures before game assets are used.

## 3. First game boot

Requires the browser-build game files supplied separately.

- Import and validate `game.wasm`, `shaders/`, and `data/`.
- Begin with a low render resolution and conservative memory settings.
- Diagnose runtime, shader, texture, and synchronization failures.
- Reach an interactive scene and verify basic movement and audio.

Acceptance: a repeatable boot into an interactive scene on the target device,
with logs recording the build, settings, and any known failures.

## 4. Playability and stability

- Add usable touch controls and controller support.
- Measure frame times, memory use, and sustained performance on the phone.
- Test extended sessions, background/resume behavior, and save handling.
- Address measured bottlenecks before extending device support or multiplayer.

Acceptance: documented device measurements and a reproducible gameplay session.
No FPS target is promised before measurements exist.
