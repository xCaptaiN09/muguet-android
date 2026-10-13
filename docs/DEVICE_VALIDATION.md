# Realme X7 Max validation

Validated on 2026-10-12 with a physical Realme X7 Max (RMX3031/cupida),
Android 16/API 36, Mali-G77 MC9. Device serials and raw screenshots/reports are
kept out of Git. The first section records the diagnostic foundation; the
October 13 game-host attempt is recorded separately below.

## Verified APK

- Package: `dev.xcaptain.muguet`
- Version: `0.1.0-foundation`, code 1
- Architecture: ARM64 only; minimum API 29, target API 36
- Source code revision: `7dcd91e`
- APK: `artifacts/muguet-android-debug.apk` (local build output, ignored by Git)
- Size: 13,921,357 bytes / 13.28 MiB
- SHA-256: `9ebf49b2620c193be927f07dbb367f08f1a2b6da96892ea86dd2ef6f5b9c7200`

APK signature verification and 16 KiB ZIP alignment checks passed. The packaged
native library has both exact JNI exports and four LOAD segments aligned to
`0x4000`. The APK contains one ARM64 native library and no game files.

## On-device results

The user ran the updated APK and exported an actual JSON report. It contains
`status: passed`, passing runtime/graphics results, and
`gameCompatibilityCertified: false`. The supplied screenshot visibly shows the
teal Vulkan frame. The controller then reran checks after relaunch and after
landscape/portrait recreation and visually confirmed successful output again.

| Check | Observed result |
| --- | --- |
| Guest atomic write/read | Wrote and read 305419896 |
| Bounded memory growth | Grew from 1 to 2 pages / 131072 bytes; third-page growth rejected |
| Cross-thread wait/notify | One waiter notified; wait result 0; main observed worker-written 591751049 |
| Graphics backend | Vulkan, integrated Mali-G77 MC9 |
| Visible output | Teal frame confirmed in actual phone screenshots |
| Surface | 984 × 540, Rgba8UnormSrgb, FIFO, not suboptimal |
| Report export | Actual JSON exported through Android's document picker |
| Relaunch/rotation | Old graphics result invalidated; running checks again restored a passed report and teal frame |
| Insets | Dark system icons and protected scrolling viewport verified in portrait and landscape |

Relevant advertised GPU capabilities:

- BC texture compression: **unsupported**
- Float32 filtering and depth-clip control: supported
- Maximum 2D texture dimension: 16384
- Maximum storage-buffer binding: 268435456 bytes / 256 MiB
- Maximum buffer size: 2147483647 bytes
- Maximum bind groups: 4

Game integration must use Muguet's BC texture decoding fallback and validate
the actual shaders against these device capabilities. Advertised limits alone
do not certify every engine pipeline.

## Regression and review checks

- Native formatting and 10 behavior tests passed, including actual guest
  atomics, memory bounds, notification, panic/error handling, blocked-cleanup
  timeout, single in-flight GPU work, and late cleanup errors.
- Nine Java report/export tests passed; Java compilation and Android lint
  completed with zero errors and five informational warnings.
- Host and Android-target Clippy passed with warnings denied during native
  task verification.
- Native timeout and cleanup-error findings were fixed and independently
  re-reviewed. UI inset and icon-contrast findings were also re-reviewed.
- Phone portrait-lock and USB stay-awake settings were restored to their
  original values after testing.

Known lint warnings concern pinned tool/dependency versions, ARM64-only
ChromeOS support, backup-rule metadata, and a launcher icon. The Android build
also reports SDK XML/Gradle deprecation notices; these did not prevent the
verified build/install. API29 behavior was compiled/reviewed, not tested on a
separate API29 device.

## Interpretation limits

`presented: true` means completed GPU work and a presentation request without
an observed API/device error. Pinned wgpu does not expose physical scanout
confirmation; the visible phone screenshots supply that separate evidence.
If a driver stalls, callers time out while one gated worker retains resources
until cleanup completes; repeated checks cannot accumulate extra GPU workers.

The Wasm probe uses only two 64 KiB pages. It does not validate GTA V's memory
footprint, engine imports, shaders, controls, or gameplay. No game frame rate,
full-game compatibility, or final installation size has been established.

## Game-host startup attempt — October 13

The game-host feature branch imports Muguet
`ddcc54f44ae2a21b8c6e43ec1c925b55b352ffbd` and uses Wasmtime 49.0.2, wgpu
30.0.1/Vulkan, and CPAL/AAudio. Execution and asset reads are local to the phone.
There is no remote gameplay or asset server in this test.

The tested APK was version `0.2.0-game-host` / code 2, SHA-256
`a1c527939001670707e98de75bc15e798e682e1db3e8d0355f1a408cf3d65dbe`.
The original diagnostic APK above remains retained separately.

The Android precompiled artifact is 210,804,184 bytes, SHA-256
`d9b0341855b8a2197d8aaeb81e7bc8ae71dd6785c1168e632dc77729888631e9`.
It was compiled for generic `aarch64-linux-android` with a matching 5 GiB heap
cap/reservation, 64 KiB guard, non-moving shared memory, memory64 and threads.
The actual module imports a minimum 3 GiB shared heap. The APK pins the artifact
digest and verifies its app-private copy before deserialization.

### Installed content and access

All 14 original archives and all 12,482 extracted PC mirror files were
checksum-verified and retained. The selected Android tree contains 12,156 files
and 15,650,794,222 bytes after deliberately filtering optional content.

ADB completed the selected-content transfer. A complete phone relative-path and
file-size comparison matched the prepared PC tree with no missing, extra, or
wrong-size files. Bounded phone SHA checks also matched the manifest, shader
index, original 63.2 MB Wasm and 210.8 MB compiled artifact. A full phone content
hash sweep was interrupted when the phone froze and is **not** certified.

The ROM denies `run-as`; the app instead copies staged code into its private
files directory. Initial launches failed with `EACCES` because ADB-created nested
asset directories were mode 2770. Granting ordinary traversal/read access on only
the managed asset directories allowed loading. The installer now normalizes
these directory permissions for reused, adopted, repaired and transferred trees.
No root, SELinux changes, broad storage permission or phone settings were used.

### Observed startup and failure

- The user's October 13 10:58:56 screenshot visibly shows GTA V's actual
  legal/startup screen. This confirms real game rendering on the physical phone.
- A final exported status recorded `present_requested`, 504 presents,
  zero observed GPU validation errors, and a 3,221,225,472-byte guest heap.
- Android's historical exit report records the foreground `:game` process
  killed at 10:59:09 with reason 3, `LOW_MEMORY`, and RSS reported as 2.9 GB.
- A subsequent user retry ended at 11:06:46 with the same `LOW_MEMORY` reason,
  RSS reported as 2.2 GB, and a last exported count of 670 presents.
- Native logs contain audio mixer output and world/resource streaming activity.
  These do not establish audible output or a visibly interactive world.

The process RSS samples and final Android exit values are snapshots, not measured
allocation peaks. The 3 GiB guest heap size is its addressable memory, not the
same metric as resident RAM. Presentation rates during startup are not sustained
gameplay FPS. Exported status files persist after process death and must not be
mistaken for a currently running session.

A conservative graphics preset and proper mobile controls are the next test.
Their effectiveness must be demonstrated in a fresh session. Playable-world
rendering, movement, vehicle actions, audible sound, save handling and sustained
stability remain unverified.

## First visible open world — October 13 follow-up

Installed version `0.3.0-mobile`, source `988cdfa`, APK SHA-256
`d941256eeeb83d6a2f1ccfcff544fbba5e511e3442cfefaac3ce4bbc2d12c48e`.
The installer reused the matching complete asset receipt and pinned code, and
successfully applied directory permission normalization. No bulk retransfer or
AOT recompilation was needed.

The user reported reaching the open world. A subsequent timed controller run
captured `artifacts/task-4/timed-world.png` locally: a standing character, rendered
city buildings and plaza, the minimap and new circular touch HUD are all visible.
This establishes real open-world rendering on the physical Realme X7 Max.
It does not yet verify movement, simultaneous world actions, vehicle controls,
audible sound, sustained frame rate or stability.

The earlier follow-up session ended with the controller's Back command at
12:11:04; its game exit was SIGNALED 9, and the launcher also had a LOW_MEMORY
record. That closure is not a confirmed new foreground game OOM. The later
timed run's game process disappeared around 55–58 seconds, before its scheduled
85-second stop. Its exit reason remains to be retrieved after reconnection;
do not assume that closure was the scheduled stop. The phone was disconnected
when work was paused at the user's request.

Exported status and native logs also record world-shader parsing/validation
errors (13–17 in these runs). Renderer compatibility and memory need further
investigation. A subsequent UI-only fix `fbbc67e` improves held-button contrast
and inset-aware Info placement; that APK is built and locally checked, but is
not yet installed or independently re-reviewed. See the resume checkpoint.
