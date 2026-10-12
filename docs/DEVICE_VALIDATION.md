# Realme X7 Max foundation validation

Validated on 2026-10-12 with a physical Realme X7 Max (RMX3031/cupida),
Android 16/API 36, Mali-G77 MC9. Device serials and raw screenshots/reports are
kept out of Git. This record covers the diagnostic foundation, not GTA V.

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
