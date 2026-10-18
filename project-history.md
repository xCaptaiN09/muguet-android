# Muguet Android — project history and continuation handoff

Last updated: **2026-10-18**. Maintainer: **xCaptaiN09**.

This file is intended to be given directly to a new contributor or AI coding
assistant. It summarizes the repository, verified evidence, unresolved work and
safe resume order without requiring the original chat or private agent logs.

## Read this first

We are adapting [c22dev/Muguet](https://github.com/c22dev/muguet) to Android so a
compatible GTA V browser build executes **on the smartphone itself**. There is
no remote gameplay stream. Local asset reads, Wasmtime, Vulkan and AAudio are
already integrated. Future asset hosting is optional and is not implemented.

**The actual open world rendered on the target phone.** A captured frame shows
the character standing in a city plaza with buildings and the minimap, alongside
the Android touch HUD. That is stronger evidence than a boot logo or a synthetic
test. It is still not proof of stable, fully interactive gameplay.

Do not restart the port from scratch. The next work is focused runtime diagnosis,
review and device testing. The latest changes described here are documentation
and completion of a private recovery backup; game bug fixing remains paused.

## Repository and target

- Repository: <https://github.com/xCaptaiN09/muguet-android>.
- Visibility: **public** (made public on October 18, 2026).
- Active/default source branch: `feat/android-game-host`, the public landing
  branch. `main` contains the original initialization, not the game host.
  No feature-to-main merge has been performed.
- Target tested: Realme X7 Max / RMX3031, Dimensity 1200, Mali-G77 MC9, 8 GB RAM,
  Android 16/API 36. New ROMs and other devices need fresh validation.
- Android package: `dev.xcaptain.muguet`; ARM64 only, minimum API 29, target API 36.
- The owner has a rooted device, but game installation/execution does not require
  root. A later private-data backup used read-only root access; that is separate
  from running the game.
- Project source license: **GPL-3.0-or-later**. Keep upstream attribution and see
  [NOTICE](NOTICE). The license does not cover proprietary GTA V assets.

## Architecture and file map

| Location | Responsibility |
| --- | --- |
| `game-host/` | Muguet-derived Wasmtime runtime, engine imports, local filesystem/I/O, GPU command handling, audio and input. |
| `game-host/src/session.rs` | One-session launch contract, requested graphics arguments, status and suspend/input boundary. |
| `game-host/src/code.rs` | Matching compiler/runtime configuration and compiled-code verification. |
| `game-host/src/gpu.rs` | Vulkan renderer, shader preparation, texture fallbacks and validation reporting. |
| `game-host/src/input.rs`, `pad.rs`, `mp.rs` | Guest input block, touch horn and script-tick vehicle context. Networking remains disabled for the current launch. |
| `native/src/game_android.rs` | JNI game bridge, Android context/audio setup, owned surface handoff and private compiled-code loading. |
| `native/` diagnostic code | Independent synthetic runtime and Vulkan probes retained alongside the game host. |
| `android/.../MainActivity.java` | Launcher and diagnostic report/export flow. |
| `android/.../GameActivity.java` | Unexported landscape activity in process `:game`, code staging, status/log export and lifecycle. |
| `android/.../MobileControls*.java` | Pure input controller, measured safe-area layout, Canvas HUD and contrast palette. |
| `scripts/prepare-game.py` | Separate prepared content tree, manifest/DLC filtering and source preservation. |
| `scripts/install-game.sh`, `content-receipt.py` | USB installation, complete relative-path/size comparison, content receipts, directory access and AOT staging. |

Java source paths abbreviated above are under
`android/app/src/main/java/dev/xcaptain/muguet/`.

The game process owns one session. Backgrounding or leaving the activity releases
controls and ends that process; reopening starts another. Do not describe this
as seamless pause/resume. `SurfaceHandoff` must retain the acquired native window
until after the Vulkan surface is dropped.

## Pinned source and tools

Muguet was initially audited at `a48e9a71e13c7ae4b161f72ea0bff868b85588c7`.
The imported engine is pinned to
**`ddcc54f44ae2a21b8c6e43ec1c925b55b352ffbd`**, retaining the upstream movie preload,
scenario collision, pipeline wait, BC decode and minimap fixes.
[Provenance](third_party/muguet/UPSTREAM.md) and `SOURCE.sha256` record the input.
The upstream vendor/wgpu-core tracker optimization is documented but not activated.

The separately supplied browser snapshot was audited through
`alihammoud21/playgta5` at `4a0d596d9b030be20811f0d5c8882f1e2ce05c07`.
That is provenance, not a redistribution license. No original GTA C++ source,
proprietary browser distribution or game asset pack is part of this repository.

Keep Rust **1.99.0**, Wasmtime **49.0.2**, wgpu **30.0.1**, CPAL **0.18.2**,
JDK **21**, Gradle **8.13**, AGP **8.13.2**, Android SDK/target **36**, Build Tools
**36.0.0** and NDK **28.2.13676358** unless a separate change deliberately updates
and validates them. The build host used is Linux x86-64.

## What happened, in order

### October 12 — foundation, assets and engine integration

1. Created the repository and initial roadmap (`8743a34`). Reviewed the iOS/native
   Muguet approach and browser snapshot; selected local execution as the goal.
2. Built the diagnostic foundation: memory64/shared-memory atomics, bounded growth,
   cross-thread wait/notify, JNI, Vulkan test frame and Android report export.
   Device/lifecycle fixes and reviews culminated in `6962201`.
3. Verified the foundation on the physical Realme: runtime probes passed and a teal
   Vulkan frame was visibly rendered. The GPU advertises **no BC texture support**,
   so the real renderer needs CPU texture decoding.
4. Downloaded and checksum-verified 14 original release archives and the complete
   extracted PC mirror, retaining independent originals outside Git. The selected
   Android tree was prepared separately and transferred over USB.
5. Imported/adapted the real Muguet engine and Android precompiler (`56b5ac3`), then
   fixed permanent audio and Vulkan loss reporting (`11ef9f1`).
6. Compiled actual Android ARM64 game code on the PC. The artifact is not an iOS
   binary and must use the same Wasmtime configuration as the Android loader.
7. Added the isolated Android game activity/JNI/staging/installer (`feeee21`).

### October 13 — access fixes, first output and mobile HUD

8. Fixed manifest/DLC validation, bounded log export and startup focus handling
   (`ee09110`), then interrupted installer receipt recovery (`1070dd4`).
9. Real launches initially failed with `EACCES`: ADB-created nested asset directories
   were mode 2770 and not traversable by the app. Normalizing read/traverse access
   on managed asset directories fixed loading; the installer makes it reproducible
   (`2c804ca`). No SELinux changes were used.
10. The real legal/startup screen rendered. Earlier foreground game processes were
    later killed by Android with reason `LOW_MEMORY`; reported RSS snapshots were
    2.9 GB and 2.2 GB. These are not measured allocation peaks.
11. Replaced the temporary gray button row with Muguet-style mobile controls and a
    conservative graphics request (`988cdfa`). Added mouse fire/aim, horn, vehicle
    context, simultaneous pointer roles, 120 ms holds and lifecycle cancellation.
12. The owner reported reaching the world. A timed physical-device screenshot
    subsequently confirmed the character, city/plaza and minimap with the new HUD.
13. Independent HUD review found pressed-button contrast and asymmetric-cutout Info
    overlap. `fbbc67e` fixes both. Its rebuilt APK is locally checked but has not
    received the pending scoped re-review or fresh device visual validation.
14. Saved the open-world checkpoint and pushed all source (`1077565`). Work paused
    at the owner's request. Runtime/shader/stability work remains open.

### October 18 — backup and public contributor handoff

15. Reconfirmed that the game-host worktree was clean and matched GitHub at
    `1077565`. The separate foundation worktree was also clean.
16. Reverified all 14 original archive SHA-256 hashes and both inventory/manifest
    hashes. Created a compressed original-archive backup and streamed it back
    through all 14 hash checks to verify recoverability.
17. Reconnected the current-ROM phone. Its 12,156-file asset path/size map matches
    the prepared PC tree, and its staged compiled-code SHA-256 matches the pin.
    This was not a full phone asset-content hash sweep.
18. Exported Muguet's private app data using read-only root access because this ROM
    denies `run-as`. Verified the four files under its save/settings directory
    against the exported copy (2,405 bytes total). This is not proof that a complete
    story-progress save exists. Also retained the installed APK and external logs.
19. Prepared a complete Git bundle and PC recovery kit locally. These were later
    included in the private cloud backup described below; no original game files
    were deleted.
20. Published the contributor README, this handoff, attribution/license metadata and
    source-only public repository requested by the owner. Set the default branch
    to `feat/android-game-host` without merging into `main`. The scope excludes
    private saves, raw captures, APKs and proprietary game assets.
21. Completed the owner's private Bacloud recovery backup via a two-core US East
    Codespace. All 14 original ZIPs were checked against their retained source
    SHA-256 values after download and by reading the uploaded bytes back. The
    complete payload contains **39 files / 21,407,262,865 bytes**: 14 original
    archives, 23 recovery/metadata files and two exact prepared-content overrides.
    All received full SHA-256 readback verification. `SHA256SUMS.txt` and
    `REMOTE-VERIFICATION.json` were uploaded and independently checked, followed
    by a fresh existence/size check of all 39 payloads at 09:16 UTC. The receipt
    records completion at 09:14 UTC on October 18. The Codespace stop request
    succeeded; no public asset mirror was created and no local originals deleted.

The first transfer stopped at its configured time limit after 10 verified parts.
Its `/tmp` worker/log files were cleared when the Codespace stopped. The resumed
worker used a private directory under persistent `/workspaces`, outside the Git
checkout, reverified the existing parts and uploaded the remaining four. Preserve
resumable state in persistent storage for any future transfer. Backup scripts and
credentials are operational data outside the source repository.

The exact current documentation commit is available from `git log -1`; this file
does not try to embed its own future commit hash.

## Exact build and content identities

| Item | Identity |
| --- | --- |
| Foundation APK | `0.1.0-foundation`, code 1; SHA-256 `9ebf49b2620c193be927f07dbb367f08f1a2b6da96892ea86dd2ef6f5b9c7200`. |
| Startup-only tested APK | `0.2.0-game-host`, code 2; SHA-256 `a1c527939001670707e98de75bc15e798e682e1db3e8d0355f1a408cf3d65dbe`. |
| Open-world tested APK | Source `988cdfa`, `0.3.0-mobile`, code 3; 16,882,653 bytes; SHA-256 `d941256eeeb83d6a2f1ccfcff544fbba5e511e3442cfefaac3ce4bbc2d12c48e`. Still installed when exported October 18. |
| Latest built APK | Source `fbbc67e`, `0.3.0-mobile`, code 3; 16,945,913 bytes; SHA-256 `e104df0e6c8fc67bfbfa3e2c818f784d814158cb9d8ae893b78adea48bfeb075`. UI fix build, not yet device-validated. |
| Android compiled game code | 210,804,184 bytes; SHA-256 `d9b0341855b8a2197d8aaeb81e7bc8ae71dd6785c1168e632dc77729888631e9`. |
| Original browser Wasm | 63,201,802 bytes; SHA-256 `11ca8d2c04c5e843d18ff4aea4899d72c86973c6b031df334e67c446b2ae83e0`. |
| Original archives | 14 files, 21,192,696,051 bytes. |
| Extracted mirror | 12,482 mirror files, 21,158,604,965 bytes. |
| Prepared Android content | 12,156 files, 15,650,794,222 bytes; optional `env_test`/`verityradio` omitted. |

These source/artifact hashes identify previous builds; they are not substitutes
for checking current bytes. A newly compiled artifact may have another digest.
The build embeds the actual artifact digest and installation must match that APK.

The AOT/runtime contract uses generic `aarch64-linux-android`, a **5 GiB cap and
reservation**, **64 KiB guard**, non-moving shared memory, memory64 and threads.
The actual guest imports a minimum **3 GiB** shared heap. Reducing a virtual
reservation is not the same as reducing resident RAM; do not blindly lower it
below the guest import or change one side without rebuilding the paired artifact.

Current requested launch: 960×432 landscape, Sandbox, offline, approximately a
30 FPS limit. The profile adds `textureQuality=0`, `shaderQuality=1`,
`grassQuality=0`, `reflectionQuality=0`, `shadowQuality=1`, `lodScale=0.35`,
`particleQuality=0` and `waterQuality=0`. Do not use shaderQuality=0 (upstream
crash restriction) or shadowQuality=0 (outside the documented range). These are
requested arguments, not proof of settings honored or a memory fix.

## Known failures and evidence limits

- **Low memory:** earlier game exits are confirmed `LOW_MEMORY`. Whole-device
  freezes were also reported after launching the old build. Do not label every
  closure a crash: the controller's Back at 12:11:04 ended one game session, and
  force-stop/background exits are expected in this prototype.
- **Last timed run:** its game process disappeared around 55–58 seconds, before
  the scheduled 85-second stop. The exit reason was not retrieved before USB
  disconnection. Check persisted exit information if still available; otherwise
  reproduce with fresh process/timestamp tracking.
- **World shaders:** status recorded 13–17 GPU errors. Logged examples include
  breakable glass, cloud and vehicle shaders. Debug formatting prints a large WGSL
  source before the cause, and truncation hides useful error details. Diagnose
  that path before applying broad shader rewrites.
- **Memory costs:** BC CPU decoding is necessary on this GPU and worth profiling;
  it is not yet established as the cause of any specific termination.
- **Output is not gameplay certification:** synthetic probes, present counts,
  startup rates and a visible world frame do not prove movement, vehicle control,
  audible sound, saves or sustained play. Full engine/branch review remains open.
- **Storage:** this ROM denies `run-as` due to SELinux context lookup. The app
  itself copies staged code into private storage and exports bounded logs. Do not
  weaken SELinux to make the installer work.
- **Preparation:** unchanged prepared files may be hardlinks. Never edit them in
  place; copy before modifying. Full phone asset hashing previously froze the
  device; use metadata plus bounded checks first.
- **Partial review status:** engine/backend and initial Android-host findings
  were fixed/re-reviewed. HUD fix `fbbc67e` needs scoped re-review and real visuals;
  the planned whole-branch runtime review is not complete.

## Where to continue

1. Read this file, [GAME_HOST.md](docs/GAME_HOST.md), current `git status`/`git log`
   and the relevant source. Preserve existing work. Do not redispatch/reimplement
   the completed runtime foundation, host import or Android staging integration.
2. If a ROM was changed, repeat the synthetic runtime/Vulkan checks and collect
   current GPU capabilities. Prior results belong to the old tested ROM.
3. Check the two HUD fixes in `205b05e..fbbc67e`: readable default/pressed/selected
   colors over bright backgrounds, and the actual shared inset-aware Info geometry.
   Focused tests cover asymmetric safe widths 495/496/497 dp. Review then install
   the latest APK without clearing preserved app data unnecessarily.
4. Capture fresh session PID, status, memory and Android exit reason together.
   `game-status.json` persists after death and must not be read as a live-session
   indicator. Use a bounded test with an independent stop deadline and log when
   the controller itself issues Back or force-stop.
5. Improve/reproduce shader diagnostics and investigate specific parsing failures.
   Preserve original shader packs and separate host compatibility changes.
6. Measure actual memory use/texture costs, then verify world movement, look/fire,
   vehicle actions, sound and stability. Change one evidenced bottleneck at a time.
7. Complete relevant reviews, record measured results and update this handoff.
   Source publication is not a claim that runtime acceptance checks are finished.

## Commands and boundaries for a new checkout

See [TESTING.md](docs/TESTING.md) for prerequisites. Commands that need no game data:

```bash
bash scripts/setup-rust.sh
source scripts/env.sh
export CARGO_BUILD_JOBS=2 RAYON_NUM_THREADS=2
cargo fmt --all --check
cargo test --locked --workspace
python3 -m unittest discover -s scripts -p 'test_*.py'
(cd android && bash gradlew --no-daemon :app:testDebugUnitTest :app:lintDebug)
```

Actual content preparation/precompile/build/install commands are in
[GAME_HOST.md](docs/GAME_HOST.md). Keep at most two PC build jobs initially;
the original development host had limited RAM. Keep compiler/runtime settings
and APK digest pin consistent. Rebuilding only UI does not require recompiling
the game artifact.

Historical verification: the pre-HUD-fix run passed 33 Rust, 22 Java and 10 Python
tests; the UI fix then passed four focused geometry/color tests and Java/lint
checks. Its APK signature and 16 KiB ZIP/all four ELF LOAD alignments were checked.
These are historical results, not a fresh test claim for an arbitrary checkout.

## Maintainer recovery data outside Git

The original developer machine uses these default locations under `$HOME`:

```text
.local/share/muguet-android/
├── game-backup/full-game-v1/       # Original ZIPs, inventories, source archives
├── game-files/full-game-v1/        # Extracted original mirror
├── prepared/full-game-v1/          # Separate Android selection
├── compiled/full-game-v1/          # Paired Android AOT code and digest
├── worktrees/game-host/            # Existing game-host checkout
├── offsite-staging/2026-10-18/      # Verified local recovery snapshot
└── backup-operations/2026-10-18/    # Private transfer logs and completion receipts
```

The October 18 staging contains the compressed originals, a full-history Git
bundle, PC recovery kit, phone app-private export, external metadata and installed
APK. The original-archive container was recovery-checked against all 14 hashes.
The private cloud copy is now verified as described above. Its `REMOTE-RESTORE.md`
explains the layout; the local 20 GB originals container was replaced there by
its 14 unmodified ZIP parts and source metadata. `prepared-overrides/` preserves
the filtered manifest and DLC list used by the Android selection. The cloud
source snapshot pins `4188569`; later handoff-only edits are in public Git history.
Older `BACKUP-STATUS.json` files describe the pre-upload snapshot; the final cloud
receipt is the completion record. A full reinstall on a new ROM has not been
tested. Local copies remain in place; any cleanup must be scoped to the intended
game data, retaining the source checkout and access to the private backup.
No Internet Archive/public game upload occurred.

Private screenshots/reports and the previous task ledger live under ignored
`artifacts/` and `.superpowers/` in that checkout. Public contributors may not have
them; this document and [DEVICE_VALIDATION.md](docs/DEVICE_VALIDATION.md) contain
the necessary evidence summary. Never upload raw app-private exports to a public
repository. New cloud credentials and ROM-specific state do not belong in Git.

## Keep this handoff current

After a meaningful session, update the date, source/build identities, observed
results, unresolved failures and exact next step. Distinguish requested settings
from measured effects, user reports from captured evidence, and implementation
from device verification. Keep older evidence dated. Record review status and
backup verification honestly so another person can resume without guessing.
