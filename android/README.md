# Muguet Android diagnostic shell

This build does not yet run GTA V. It runs synthetic native runtime checks and
presents a Vulkan test frame; even a passed report does not certify game
compatibility. Local import is the recommended future asset provider, and a
future download-as-you-play provider remains possible. Neither is implemented.

From the repository root:

```sh
source scripts/env.sh
cd android
./gradlew :app:testDebugUnitTest :app:compileDebugJavaWithJavac :app:lintDebug
./gradlew :app:assembleDebug
```

The wrapper pins Gradle 8.13 with the SHA-256 published at
https://services.gradle.org/distributions/gradle-8.13-bin.zip.sha256.
The build uses AGP 8.13.2, JDK 21 (selected by the environment script), API 36,
minimum API 29, NDK 28.2.13676358, and ARM64 only. Java bytecode targets Java 17.

APK assembly invokes `../scripts/build-native.sh` and packages its generated
ARM64 JNI library. JVM report tests use real `org.json` and do not build or load
that library. Java compilation and lint also need no native build.

Use **Run checks** while the Surface is visible, then **Export report** to save
the actual JSON with Android's document picker. Surface/Activity changes make
graphics pending until checks are run again. Native calls run on one process-wide
serial worker; owned Surface references remain alive until work completes.
Library-load failures are recorded as failed diagnostics. No Internet, root, or
storage permissions are requested, and no game files are included or downloaded.

Phone execution and document-provider behavior require device verification.
