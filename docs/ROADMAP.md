# Android port roadmap

End goal: GTA V running locally on the Realme X7 Max, with an interactive world,
touch controls, audio, and offline single-player gameplay after installation.
Diagnostics support this goal; the project is complete only when the gameplay
and stability milestones have been demonstrated on the phone.

## 0. Repository setup

Completed: local repository, private GitHub repository, and project documentation.

- Initialize the local project and Git history.
- Create the GitHub repository under `xCaptaiN09`.
- Document the target device and keep game files out of version control.

## 1. Compatibility APK

Completed on the Realme X7 Max/API 36. Runtime probes, visible Mali-G77 Vulkan
output, report export, and lifecycle/surface recreation were verified. See
[device validation](DEVICE_VALIDATION.md). This does not certify GTA gameplay.

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

Implemented on the game-host feature branch: reusable Muguet host, Vulkan,
Android-targeted precompiled code with checksum verification, AAudio, local
asset installation, logging and an isolated game process. Real startup output
has rendered on the phone; broader compatibility is still being tested.

- Review and import the necessary Muguet code with its license and attribution.
- Isolate Apple-specific dependencies and replace Metal backend selection.
- Integrate Android input, audio, file import, logging, and app lifecycle.
- Validate shaders and texture fallbacks against the device's Vulkan driver.
- Choose Android-targeted runtime compilation and caching after the runtime
  probe; do not assume an iOS precompiled artifact is portable to Android.

Acceptance: the native host runs on Android and its platform integrations work
with synthetic fixtures before game assets are used.

## 3. First game boot

In progress. Selected browser-build files and trusted Android compiled code are
installed separately. The October 13 attempt rendered the legal/startup screen,
then Android killed the game process for low memory. This is not yet an
interactive-scene acceptance pass.

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
