//! Bound the JNI caller's wait without abandoning GPU/window ownership.
use anyhow::{bail, ensure, Context};
use serde_json::Value;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

#[derive(Default)]
pub(crate) struct GpuWorker {
    active: Arc<AtomicBool>,
}

impl GpuWorker {
    pub(crate) fn run(
        &self,
        timeout: Duration,
        operation: impl FnOnce() -> anyhow::Result<Value> + Send + 'static,
    ) -> anyhow::Result<Value> {
        ensure!(
            self.active
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
            "Previous Vulkan worker is still running or cleaning up"
        );
        let permit = ActiveWorker(Arc::clone(&self.active));
        let (sender, receiver) = mpsc::sync_channel(1);
        // Dropping a JoinHandle detaches, never joins. The worker owns every GPU
        // resource until normal cleanup finishes, even after the caller times out.
        drop(
            thread::Builder::new()
                .name("muguet-vulkan".to_owned())
                .spawn(move || {
                    let report = crate::gpu::diagnostic_boundary(operation);
                    // All operation captures, GPU locals and cleanup are gone before the
                    // gate reopens. A spawn failure also drops this RAII permit.
                    drop(permit);
                    let _ = sender.send(report);
                })
                .context("Could not start Vulkan worker")?,
        );
        match receiver.recv_timeout(timeout) {
            Ok(report) => Ok(report),
            Err(mpsc::RecvTimeoutError::Timeout) => bail!(
                "Vulkan worker timed out after {} ms; it retains GPU/window ownership until cleanup finishes",
                timeout.as_millis()
            ),
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("Vulkan worker exited without a diagnostic report"),
        }
    }
}

struct ActiveWorker(Arc<AtomicBool>);

impl Drop for ActiveWorker {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(Clone, Default)]
pub(crate) struct GpuErrors(Arc<Mutex<Vec<String>>>);

impl GpuErrors {
    pub(crate) fn record(&self, error: String) {
        // Never panic inside a wgpu callback. Poisoning itself is reported by
        // check(), so even a poisoned accumulator cannot turn errors into success.
        if let Ok(mut errors) = self.0.lock() {
            errors.push(error);
        }
    }

    pub(crate) fn device_lost(&self, reason: wgpu::DeviceLostReason, message: String) {
        // wgpu also invokes this callback for expected device destruction.
        if reason != wgpu::DeviceLostReason::Destroyed {
            self.record(format!("Device lost ({reason:?}): {message}"));
        }
    }

    pub(crate) fn check(&self) -> anyhow::Result<()> {
        let errors = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("GPU error lock poisoned"))?;
        ensure!(errors.is_empty(), "GPU error: {}", errors.join("; "));
        Ok(())
    }
}

pub(crate) fn after_cleanup(
    operation: impl FnOnce(&GpuErrors) -> anyhow::Result<Value>,
) -> anyhow::Result<Value> {
    let errors = GpuErrors::default();
    // FnOnce has returned and dropped its resources before the final check.
    // Keep the accumulator alive through queue/surface/device/window teardown.
    let report = operation(&errors)?;
    errors.check()?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{sync::mpsc, thread};

    struct BlockedCleanup {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
        released: Arc<AtomicBool>,
    }

    impl Drop for BlockedCleanup {
        fn drop(&mut self) {
            let _ = self.entered.send(());
            let _ = self.release.recv();
            self.released.store(true, Ordering::Release);
        }
    }

    #[test]
    fn blocked_cleanup_times_out_caller_and_keeps_one_worker_in_flight() {
        let worker = Arc::new(GpuWorker::default());
        let released = Arc::new(AtomicBool::new(false));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let caller_worker = Arc::clone(&worker);
        let owned_resource = BlockedCleanup {
            entered: entered_tx,
            release: release_rx,
            released: Arc::clone(&released),
        };
        let caller = thread::spawn(move || {
            let result = caller_worker.run(Duration::from_millis(20), move || {
                let _resource = owned_resource;
                Ok(json!({"status":"passed", "presented":true}))
            });
            let _ = result_tx.send(result);
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let response_before_cleanup = result_rx.recv_timeout(Duration::from_millis(200));
        let still_owned = !released.load(Ordering::Acquire);
        let attempted = Arc::new(AtomicBool::new(false));
        let attempted_in_worker = Arc::clone(&attempted);
        let retry = worker.run(Duration::from_millis(20), move || {
            attempted_in_worker.store(true, Ordering::Release);
            Ok(json!({"status":"passed"}))
        });
        // Always unblock the destructor before asserting, including the RED run.
        release_tx.send(()).unwrap();
        caller.join().unwrap();
        assert!(
            response_before_cleanup.is_ok(),
            "caller waited for blocked cleanup"
        );
        let timeout = response_before_cleanup.unwrap().unwrap_err();
        assert!(timeout.to_string().contains("timed out"));
        assert!(still_owned, "resource released before cleanup finished");
        assert!(retry.unwrap_err().to_string().contains("still running"));
        assert!(
            !attempted.load(Ordering::Acquire),
            "retry started a second worker"
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while worker.active.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(!worker.active.load(Ordering::Acquire));
        assert!(released.load(Ordering::Acquire));
        assert_eq!(
            worker
                .run(Duration::from_secs(2), || Ok(json!({"status":"passed"})))
                .unwrap()["status"],
            "passed"
        );
    }

    struct CleanupError(GpuErrors);

    impl Drop for CleanupError {
        fn drop(&mut self) {
            self.0.device_lost(
                wgpu::DeviceLostReason::Unknown,
                "cleanup device failure".to_owned(),
            );
        }
    }

    #[test]
    fn cleanup_device_loss_prevents_success() {
        let result = after_cleanup(|errors| {
            let _resource = CleanupError(errors.clone());
            Ok(json!({"status":"passed", "presented":true}))
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cleanup device failure"));
    }

    #[test]
    fn normal_device_destruction_does_not_fail_diagnostics() {
        let result = after_cleanup(|errors| {
            errors.device_lost(
                wgpu::DeviceLostReason::Destroyed,
                "normal destruction".to_owned(),
            );
            Ok(json!({"status":"passed", "presented":true}))
        });
        assert_eq!(result.unwrap()["status"], "passed");
    }

    #[test]
    fn worker_contains_panics_and_accepts_the_next_probe() {
        let worker = GpuWorker::default();
        let report = worker
            .run(Duration::from_secs(2), || panic!("worker panic"))
            .unwrap();
        assert_eq!(report["status"], "failed");
        assert_eq!(report["presented"], false);
        assert_eq!(
            worker
                .run(Duration::from_secs(2), || Ok(json!({"status":"passed"})))
                .unwrap()["status"],
            "passed"
        );
    }

    #[test]
    fn uncaptured_gpu_error_prevents_success() {
        let result = after_cleanup(|errors| {
            errors.record("GPU validation failure".to_owned());
            errors.check()?;
            Ok(json!({"status":"passed"}))
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("GPU validation failure"));
    }
}
