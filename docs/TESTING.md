# Build and test Muguet Android

The app contains independent runtime/Vulkan diagnostics and an experimental local
game host. Synthetic checks need no game files or root. Real-game setup requires
separately supplied compatible content; see [GAME_HOST.md](GAME_HOST.md).
The current evidence and resume point are in [project-history.md](../project-history.md).

## Build tools

The current scripts target a Linux x86-64 development host. Install Android
SDK platform 36, build tools 36.0.0, and NDK 28.2.13676358. With Android CLI:

```bash
android sdk install platform-tools 'platforms;android-36' 'build-tools;36.0.0' 'ndk;28.2.13676358'
bash scripts/setup-rust.sh
```

Use JDK 21. The environment script selects the installed Arch Linux JDK 21
when JAVA_HOME is unset. On another installation, set JAVA_HOME to its JDK 21
directory. The scripts use an isolated Rust installation under
`~/.cache/muguet-android`; they do not replace system Rust or Java.

## Local checks

```bash
source scripts/env.sh
cargo fmt --all --check
cargo test --locked --workspace
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo run --locked -p muguet-android-native --bin check_runtime
cargo run --locked -p muguet-android-native --bin check_gpu
```

The host GPU probe can fail if Vulkan hardware/drivers are unavailable. That
is a diagnostic result, not evidence about the phone's GPU. It must never
claim that an Android frame was presented.

## Build the APK

```bash
bash scripts/build-apk.sh
```

The script builds the native library automatically, copies the debug APK to
`artifacts/muguet-android-game-debug.apk` and prints its SHA-256. Build output and
APKs are excluded from Git. Without the separately compiled game artifact, the
diagnostics remain usable and native game launch is disabled.

For Java report tests and Android lint:

```bash
source scripts/env.sh
cd android
bash gradlew --no-daemon :app:testDebugUnitTest :app:lintDebug
```

## Test on the Realme X7 Max

Install the debug APK through Android's package installer, or use ADB from
the repository root when USB debugging is connected:

```bash
adb install -r artifacts/muguet-android-game-debug.apk
```

Open Muguet Android, run the device checks, and export the JSON report using
the app's export control. Save it in a location you can share, such as
Downloads. Include the report even if checks fail; the actual error and GPU
capabilities guide the next porting step.

Repeat the checks after backgrounding and reopening the app, and after a
surface recreation such as rotating the device. A colored frame and passing
synthetic Wasm checks establish this app foundation's behavior; they do not
establish GTA V compatibility or a game frame rate.
