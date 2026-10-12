# Android app foundation

The project goal is GTA V running locally on the Realme X7 Max using Muguet's
approach. This iteration produces the Android application foundation and an
installable ARM64 diagnostic build. Game integration remains a later milestone;
the interface must state that this build does not yet run GTA V.

## Delivery direction

Support local game-file import first and keep the design open to a later
download-as-you-play option. These are two asset providers for the same local
game execution, not two game engines. This iteration does not download or host
game assets, create a cloud account, or deploy a server.

Cloudflare Pages is a candidate for static asset chunks to investigate later:
its documented Free plan allows 20,000 files and a maximum of 25 MiB per asset.
An asset server needs a separately verified chunk layout and availability.
GitHub Releases supports downloadable release archives, but those ZIP files
are not a ready-to-use individual-asset streaming endpoint.

## Application boundary

Use a small Java Android Activity with a SurfaceView and a Rust JNI library.
The app runs native checks off the UI thread, presents a visible Vulkan test
frame, shows the results, and exports a JSON report through Android's document
picker. It must handle system insets and surface recreation. It requires no
root access, internet permission, or game files for this milestone.

The Rust library exposes exactly these Java native methods in
`dev.xcaptain.muguet.NativeBridge`:

```java
public static native String runRuntimeChecks();
public static native String renderSurface(android.view.Surface surface, int width, int height);
```

The library name is `muguet_android`. Each return value is a JSON object with
`status` (`passed` or `failed`) and diagnostic details. JNI failures and caught
Rust panics must become a failed report rather than unwind across JNI.

## Runtime acceptance

Configure Wasmtime explicitly for memory64, shared memory, and Wasm threads.
Use a synthetic module with shared 64-bit memory bounded to two 64 KiB pages.
Actually read/write using guest atomic operations, exercise wait/notify across
host threads, grow within the bound, and report rejection of growth beyond it.
Use timeouts so a faulty wait test cannot hang the app indefinitely. A successful
synthetic probe does not certify the full GTA engine.

Rust interface: `runtime::run_runtime_probe() -> serde_json::Value`. The result
contains a `checks` array; entries have `name`, `status`, and `details` fields.

## Graphics acceptance

Select only the Vulkan backend. On Android obtain a referenced ANativeWindow
from the supplied Surface, retain it until GPU work and surface teardown finish,
and release it afterward. Validate dimensions before creating resources. Report
the real adapter name/backend, limits, available features, and BC texture
support. Present a colored frame; never mark presentation successful on a
headless probe. GPU JSON includes a top-level boolean `presented`, false for
host discovery and true only after successful Android presentation. The app
requires this flag to pass the combined checks. Unavailable Vulkan hardware
is an explicit failed result.

With pinned wgpu, presentation success here means a completed submission and
presentation request without an observed API/device error. Its public API does
not prove physical scanout; verify the visible frame separately on the phone.
Keep GPU work and teardown behind a bounded caller-response boundary, retaining
native-window resources until the worker actually finishes cleanup. A timeout
must not spawn an unbounded number of in-flight GPU jobs. Check cleanup-time
errors before deciding success.

Host interface: `gpu::probe_vulkan() -> serde_json::Value`, reporting adapter
discovery without claiming presentation. Android JNI owns the Surface handoff.

## Build and validation

- ARM64 Android only, minimum API 29, compile/target API 36.
- Android Gradle Plugin 8.13.2, Gradle 8.13, JDK 21.
- NDK 28.2.13676358; native library load segments aligned to at least 16 KiB.
- Rust 1.99.0, Wasmtime 49, wgpu 30.0.1, JNI 0.21.
- Do not modify system Java/Rust defaults; use the project environment script.
- No GTA binaries, shaders, or assets in Git or the APK.

Run native behavior tests, Java report tests, formatting/static checks, and an
APK assembly. Inspect the APK manifest, ARM64 native library, alignment, and
JNI exports. Install and collect a phone report if a device is connected;
otherwise deliver the built APK and state that phone execution is unverified.
