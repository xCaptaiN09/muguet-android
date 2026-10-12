//! JNI owns the acquired Android window for the entire GPU operation.
use crate::gpu;
use crate::gpu_worker::{after_cleanup, GpuErrors, GpuWorker};
use anyhow::{bail, ensure, Context};
use jni::{
    objects::{JClass, JObject},
    sys::{jint, jobject, jstring},
    JNIEnv,
};
use raw_window_handle::{AndroidDisplayHandle, AndroidNdkWindowHandle};
use serde_json::{json, Value};
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::{self, NonNull},
    sync::{Arc, LazyLock},
    time::Duration,
};

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jobject) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
}

pub(crate) struct NativeWindow(NonNull<c_void>);

// SAFETY: ANativeWindow_fromSurface returns an independently acquired native
// reference, usable from a rendering thread. Only this owner moves to the worker;
// JNI local references/JNIEnv never leave the caller, and the pointer is not shared.
unsafe impl Send for NativeWindow {}
// ANativeWindow references are thread-safe native objects; the only mutation
// through this wrapper is their final reference release after the surface drops.
unsafe impl Sync for NativeWindow {}

static GPU_WORKER: LazyLock<GpuWorker> = LazyLock::new(GpuWorker::default);
const GPU_CALL_TIMEOUT: Duration = Duration::from_secs(8);

impl NativeWindow {
    pub(crate) fn acquire(env: &JNIEnv<'_>, surface: &JObject<'_>) -> anyhow::Result<Self> {
        ensure!(!surface.is_null(), "Android Surface is null");
        // SAFETY: JNI arguments remain valid local references throughout this call.
        // fromSurface acquires one native reference on success, released by Drop.
        let pointer = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
        NonNull::new(pointer)
            .map(Self)
            .context("ANativeWindow_fromSurface failed")
    }

    pub(crate) fn create_game_surface(
        &self,
        instance: &wgpu::Instance,
    ) -> anyhow::Result<wgpu::Surface<'static>> {
        // SAFETY: the caller places this surface and an Arc owning this window
        // in SurfaceHandoff, whose field order drops the surface first.
        unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(AndroidDisplayHandle::new().into()),
                raw_window_handle: AndroidNdkWindowHandle::new(self.0).into(),
            })
        }
        .context("Could not create game Vulkan surface")
    }

    fn create_surface<'window>(
        &'window self,
        instance: &wgpu::Instance,
    ) -> anyhow::Result<wgpu::Surface<'window>> {
        // SAFETY: this acquired window stays alive for the returned surface's
        // lifetime. The borrow prevents release before the surface is dropped.
        unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(AndroidDisplayHandle::new().into()),
                raw_window_handle: AndroidNdkWindowHandle::new(self.0).into(),
            })
        }
        .context("Could not create Vulkan surface")
    }
}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        // SAFETY: exactly one reference is owned, acquired by fromSurface.
        unsafe { ANativeWindow_release(self.0.as_ptr()) };
    }
}

fn render_surface(
    env: &JNIEnv<'_>,
    java_surface: &JObject<'_>,
    width: jint,
    height: jint,
) -> anyhow::Result<Value> {
    // Reject invalid sizes before acquiring even the Android window.
    gpu::validate_dimensions(width, height, gpu::MAX_SURFACE_DIMENSION)?;
    let window = NativeWindow::acquire(env, java_surface)?;
    GPU_WORKER.run(GPU_CALL_TIMEOUT, move || {
        after_cleanup(|errors| render_window(window, width, height, errors))
    })
}

fn render_window(
    window: NativeWindow,
    width: jint,
    height: jint,
    errors: &GpuErrors,
) -> anyhow::Result<Value> {
    // The owned argument drops after all locals, including on errors/panics.
    // This worker alone owns GPU work AND potentially blocking GPU teardown.
    let instance = gpu::vulkan_instance();
    let surface = window.create_surface(&instance)?;
    let adapter = gpu::request_adapter(&instance, Some(&surface))?;
    let (width_u32, height_u32) =
        gpu::validate_dimensions(width, height, adapter.limits().max_texture_dimension_2d)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Muguet Vulkan diagnostic"),
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    }))
    .context("Could not create Vulkan device")?;
    gpu::validate_dimensions(width, height, device.limits().max_texture_dimension_2d)?;

    // Uncaptured validation, OOM, internal and device-lost errors must fail the
    // report rather than disappear or trigger wgpu's default panic callback.
    let callback_errors = errors.clone();
    device.on_uncaptured_error(Arc::new(move |error| {
        callback_errors.record(format!("{error}"));
    }));
    let callback_errors = errors.clone();
    device.set_device_lost_callback(move |reason, message| {
        callback_errors.device_lost(reason, message);
    });

    let capabilities = surface.get_capabilities(&adapter);
    ensure!(
        capabilities
            .usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT),
        "Surface cannot be a render attachment"
    );
    let mut config = surface
        .get_default_config(&adapter, width_u32, height_u32)
        .context("Vulkan surface has no supported configuration")?;
    config.present_mode = wgpu::PresentMode::Fifo;
    config.alpha_mode = *capabilities
        .alpha_modes
        .first()
        .context("Surface has no alpha modes")?;
    surface.configure(&device, &config);
    errors.check()?;
    let (frame, suboptimal) = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
        wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
        error => bail!("Could not acquire Vulkan surface frame: {error:?}"),
    };
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Muguet clear-color frame"),
    });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Visible teal diagnostic frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.025,
                        g: 0.55,
                        b: 0.45,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    errors.check()?;
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(5)),
        })
        .context("Vulkan clear-color submission did not complete within five seconds")?;
    errors.check()?;
    queue.present(frame);
    device
        .poll(wgpu::PollType::Poll)
        .context("Vulkan poll after presentation failed")?;
    errors.check()?;

    let mut report = gpu::adapter_report(&adapter);
    report["presented"] = json!(true);
    report["surface"] = json!({
        "width": width_u32,
        "height": height_u32,
        "format": format!("{:?}", config.format),
        "presentMode": format!("{:?}", config.present_mode),
        "suboptimal": suboptimal,
        "clearColor": [0.025, 0.55, 0.45, 1.0]
    });
    report["details"] = json!("Clear-color GPU work completed and wgpu presentation requested without a reported error; visible output requires device observation");
    // Provisional result: after_cleanup checks callbacks again only after this
    // function has dropped every GPU resource and released the acquired window.
    Ok(report)
}

fn throw_if_no_exception(env: &mut JNIEnv<'_>, message: &str) {
    // Keep any Java exception already raised by fromSurface or new_string,
    // particularly OutOfMemoryError. JNI allocation failure is never success.
    if matches!(env.exception_check(), Ok(false)) {
        let _ = env.throw_new("java/lang/RuntimeException", message);
    }
}

fn jni_report(
    env: &mut JNIEnv<'_>,
    operation: impl FnOnce(&JNIEnv<'_>) -> anyhow::Result<Value>,
) -> jstring {
    // Outer containment includes JSON serialization and JNI string creation;
    // the diagnostic boundary translates operation panics into failed JSON.
    match catch_unwind(AssertUnwindSafe(|| {
        let report = gpu::diagnostic_boundary(|| operation(env));
        match env.exception_check() {
            Ok(true) => return ptr::null_mut(),
            Err(error) => {
                throw_if_no_exception(env, &format!("Could not inspect JNI exception: {error}"));
                return ptr::null_mut();
            }
            Ok(false) => {}
        }
        match env.new_string(report.to_string()) {
            Ok(string) => string.into_raw(),
            Err(error) => {
                throw_if_no_exception(
                    env,
                    &format!("Could not allocate diagnostic JSON string: {error}"),
                );
                ptr::null_mut()
            }
        }
    })) {
        Ok(result) => result,
        Err(_) => {
            throw_if_no_exception(
                env,
                "Caught Rust panic while returning native diagnostic JSON",
            );
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_runRuntimeChecks(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    jni_report(&mut env, |_| Ok(crate::runtime::run_runtime_probe()))
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_renderSurface(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    surface: JObject<'_>,
    width: jint,
    height: jint,
) -> jstring {
    jni_report(&mut env, |env| render_surface(env, &surface, width, height))
}
