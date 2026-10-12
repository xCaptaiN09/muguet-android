//! Vulkan discovery shared by the host CLI and Android presentation path.
use anyhow::Context;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// A conservative pre-allocation bound, followed by the real adapter/device limit.
#[cfg(any(target_os = "android", test))]
pub(crate) const MAX_SURFACE_DIMENSION: u32 = 16384;

#[cfg(any(target_os = "android", test))]
pub(crate) fn validate_dimensions(
    width: i32,
    height: i32,
    limit: u32,
) -> anyhow::Result<(u32, u32)> {
    anyhow::ensure!(
        width > 0 && height > 0,
        "Surface dimensions must be positive: {width}x{height}"
    );
    anyhow::ensure!(
        width as u32 <= limit && height as u32 <= limit,
        "Surface dimensions {width}x{height} exceed limit {limit}"
    );
    Ok((width as u32, height as u32))
}

/// Catches native errors and unwinding panics before they reach a JNI caller.
pub(crate) fn diagnostic_boundary(operation: impl FnOnce() -> anyhow::Result<Value>) -> Value {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(report)) => report,
        Ok(Err(error)) => {
            json!({"status": "failed", "presented": false, "details": format!("{error:#}")})
        }
        Err(panic) => {
            let message = panic
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string panic payload");
            json!({"status": "failed", "presented": false, "details": format!("Caught Rust panic: {message}")})
        }
    }
}

pub(crate) fn vulkan_instance() -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::VULKAN;
    wgpu::Instance::new(descriptor)
}

pub(crate) fn request_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> anyhow::Result<wgpu::Adapter> {
    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: surface,
        // Report hardware capabilities rather than WebGPU's bucketed defaults.
        apply_limit_buckets: false,
        ..Default::default()
    }))
    .context("No compatible Vulkan adapter available")
}

pub(crate) fn adapter_report(adapter: &wgpu::Adapter) -> Value {
    let info = adapter.get_info();
    let features = adapter.features();
    let limits = adapter.limits();
    json!({
        "status": "passed",
        "presented": false,
        "adapter": {
            "name": info.name,
            "backend": format!("{:?}", info.backend),
            "deviceType": format!("{:?}", info.device_type),
            "vendor": info.vendor,
            "device": info.device,
            "driver": info.driver,
            "driverInfo": info.driver_info
        },
        "features": features.iter_names().map(|(name, _)| name).collect::<Vec<_>>(),
        "bcTextureSupport": features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        "limits": {
            "maxTextureDimension2D": limits.max_texture_dimension_2d,
            "maxBufferSize": limits.max_buffer_size,
            "maxStorageBufferBindingSize": limits.max_storage_buffer_binding_size,
            "maxBindGroups": limits.max_bind_groups,
            "maxComputeWorkgroupSizeX": limits.max_compute_workgroup_size_x,
            "maxComputeInvocationsPerWorkgroup": limits.max_compute_invocations_per_workgroup,
            "all": format!("{limits:?}")
        }
    })
}

/// Discovers Vulkan without a display; this never claims a frame was presented.
pub fn probe_vulkan() -> Value {
    diagnostic_boundary(|| {
        let instance = vulkan_instance();
        let adapter = request_adapter(&instance, None)?;
        let mut report = adapter_report(&adapter);
        report["details"] =
            json!("Vulkan adapter discovery only; no surface or presentation on host");
        Ok(report)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_dimensions_before_resource_creation() {
        for (width, height) in [
            (0, 1),
            (1, 0),
            (-1, 1),
            (1, -1),
            (16385, 1),
            (1, 16385),
            (i32::MAX, i32::MAX),
        ] {
            assert!(
                validate_dimensions(width, height, MAX_SURFACE_DIMENSION).is_err(),
                "accepted {width}x{height}"
            );
        }
        assert!(validate_dimensions(4097, 1080, 4096).is_err());
        assert_eq!(
            validate_dimensions(1080, 2400, 16384).unwrap(),
            (1080, 2400)
        );
        assert_eq!(validate_dimensions(4096, 4096, 4096).unwrap(), (4096, 4096));
    }

    #[test]
    fn native_errors_become_failed_json_without_presentation() {
        let result = diagnostic_boundary(|| anyhow::bail!("surface lost: \"bad\"\nretry"));
        assert_eq!(result["status"], "failed");
        assert_eq!(result["presented"], false);
        assert!(result["details"].as_str().unwrap().contains("surface lost"));
        assert_eq!(
            serde_json::from_str::<Value>(&result.to_string()).unwrap(),
            result
        );
    }

    #[test]
    fn native_panics_become_failed_json_without_presentation() {
        let result = diagnostic_boundary(|| panic!("driver panic"));
        assert_eq!(result["status"], "failed");
        assert_eq!(result["presented"], false);
        assert!(result["details"].as_str().unwrap().contains("driver panic"));
    }

    #[test]
    fn successful_diagnostics_preserve_actual_result() {
        let result = diagnostic_boundary(|| {
            Ok(serde_json::json!({"status":"passed", "presented":false, "adapter":"real"}))
        });
        assert_eq!(result["adapter"], "real");
        assert_eq!(result["presented"], false);
    }
}
