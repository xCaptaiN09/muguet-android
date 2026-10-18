# Resume checkpoint — October 13, 2026

Historical checkpoint. See [project-history.md](../project-history.md) for the
current contributor/AI handoff, including October 18 backup and publication work.

**October 18 cleanup update:** the private cloud backup is verified, the phone app
is uninstalled and the large local assets/build caches were removed. Source,
evidence and small recovery kits are retained. Restore content and matching AOT
code before real-game testing. The October 13 details below are historical.

Paused at the user's request after the first actual open-world screenshot.
Private working branch: `feat/android-game-host`; no main merge or public release.

## Achieved

- GTA V executes locally on the Realme X7 Max; a physical-device screenshot
  shows the character, city/plaza, minimap and circular mobile HUD.
- Muguet-derived joystick, camera drag, contextual action and menu controls
  replace the provisional gray row. Controller/native input tests pass.
- Conservative graphics arguments are requested. Their effect on sustained
  stability is not established. Earlier builds were killed for LOW_MEMORY.
- Original release archives, complete mirror, prepared content and compiled
  Android code remain backed up locally outside Git.

## Saved implementation and artifacts

- `988cdfa`: mobile controls, native input/context and conservative preset.
- `fbbc67e`: fix pressed-control contrast and Info cutout placement. Focused
  tests, compilation/lint, APK signature and 16 KiB ZIP/ELF checks passed.
  Independent scoped re-review and fresh device visuals are pending.
- Latest built APK: `artifacts/muguet-android-game-debug.apk`, 16,945,913 bytes,
  version 3 / `0.3.0-mobile`, SHA-256
  `e104df0e6c8fc67bfbfa3e2c818f784d814158cb9d8ae893b78adea48bfeb075`.
- Phone last installed the preceding `d941256e…` APK. Install the newer build
  without clearing app data when testing resumes.
- Compiled-code pin remains
  `d9b0341855b8a2197d8aaeb81e7bc8ae71dd6785c1168e632dc77729888631e9`.
  No guest/AOT configuration or original/hardlinked asset changes.
- Local device evidence: `artifacts/task-4/timed-world.png`, `timed-monitor.jsonl`,
  `hud-first.png`, `hud-exit-info.txt`, native/status logs. These are ignored by
  Git along with APKs and game files; the source/documentation checkpoint is pushed.
- Local controller ledger/reports:
  `.superpowers/sdd/2026-10-12-android-game-host/` in the game-host worktree.

## Resume in order

1. Read the existing ledger; do not redispatch completed Tasks 1/2 or Task 4's
   implementation. Obtain scoped re-review of `205b05e..fbbc67e` for the two HUD
   findings; reports and original review are in the local SDD directory.
2. Reconnect the unlocked phone. Retrieve historical exit information for the
   last timed game run, which disappeared before the scheduled 85-second stop.
   Previous controller Back/force-stop exits must be distinguished from crashes.
3. Install the latest APK with the existing installer and reuse assets/code.
   Verify revised normal/pressed HUD and cutout placement on the real device.
4. Investigate recorded world-shader parsing errors and measure fresh session
   memory. Shader error logging currently truncates the useful cause behind WGSL
   source text; reproduce/diagnose before changing renderer behavior.
5. Test actual movement, camera/actions, vehicles, sound, stability and isolated
   process lifecycle. Current screenshot proves world rendering only.
6. Update evidence, complete scoped fixes/reviews and the planned whole-branch
   review. Push the private feature branch; no main merge was requested.

Use bounded phone tests with an independent stop deadline. Do not repeat the
full 15 GB phone hash sweep; metadata and small bounded hashes already verified
the installed staging. Do not use root or change SELinux/system settings.
