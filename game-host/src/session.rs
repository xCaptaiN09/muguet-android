use anyhow::{Context, Result};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
#[derive(Clone, Debug)]
pub struct LaunchConfig {
    pub game_root: PathBuf,
    pub code_path: PathBuf,
    /// Trusted compiler artifact digest embedded by the APK build. Never populate
    /// this from a checksum beside downloaded/user-writable native code.
    pub expected_code_sha256: String,
    pub saves_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub width: u32,
    pub height: u32,
}
// Fields drop in declaration order: the Vulkan surface drops before its owner.
pub struct SurfaceHandoff {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub owner: Arc<dyn Send + Sync>,
}
struct SessionGate(AtomicBool);
impl SessionGate {
    fn accept(&self) -> Result<()> {
        self.0
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| anyhow::anyhow!("one game session per process; restart the game process"))
    }
}
static ACCEPTED: SessionGate = SessionGate(AtomicBool::new(false));
#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Idle,
    Accepted,
    ModuleReady,
    Running,
    PresentRequested,
    Failed,
}
impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Accepted => "accepted",
            Self::ModuleReady => "module_ready",
            Self::Running => "running",
            Self::PresentRequested => "present_requested",
            Self::Failed => "failed",
        }
    }
}
static STATE: Mutex<(Phase, Option<String>)> = Mutex::new((Phase::Idle, None));
static GPU_ERRORS: AtomicU64 = AtomicU64::new(0);
pub(crate) fn phase(p: Phase) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if s.1.is_none() {
        s.0 = p;
    }
}
pub(crate) fn fail(e: impl Into<String>) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if s.1.is_none() {
        s.0 = Phase::Failed;
        s.1 = Some(e.into());
    }
}
pub(crate) fn gpu_error() {
    GPU_ERRORS.fetch_add(1, Ordering::Relaxed);
}
pub(crate) fn guard(name: &str, f: impl FnOnce()) {
    if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        let detail = p
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| p.downcast_ref::<&str>().copied())
            .unwrap_or("unknown panic");
        fail(format!("{name} thread panicked: {detail}"));
    }
}
pub fn status() -> serde_json::Value {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    serde_json::json!({"phase":s.0.as_str(),"error":s.1,"presents":crate::gpu::PRESENTS.load(Ordering::Relaxed),"gpuErrors":GPU_ERRORS.load(Ordering::Relaxed),"heapBytes":crate::RT.get().map(|rt|rt.mem.data_size()).unwrap_or(0)})
}
pub fn suspend(paused: bool) {
    crate::gpu::SUSPENDED.store(paused, Ordering::Relaxed);
    crate::input::set_focus(!paused);
}
pub fn key(code: &str, pressed: bool) {
    crate::input::key(code, pressed, false, None, false);
}
pub fn mouse_delta(dx: f64, dy: f64) {
    if dx.is_finite() && dy.is_finite() {
        crate::input::mouse_delta_f(dx, dy);
    }
}
pub fn launch(config: LaunchConfig, surface: SurfaceHandoff) -> Result<()> {
    crate::platform::validate_dimensions(config.width, config.height)?;
    ACCEPTED.accept()?;
    phase(Phase::Accepted);
    let spawn = std::thread::Builder::new()
        .name("engine-main".into())
        .stack_size(64 << 20)
        .spawn(move || {
            guard("engine-main", || {
                let run = || -> Result<()> {
                    let build = crate::platform::discover_build(&config.game_root)?;
                    std::fs::create_dir_all(&config.saves_dir)?;
                    std::fs::create_dir_all(&config.cache_dir)?;
                    std::env::set_var("MUGUET_FSROOT", &config.saves_dir);
                    std::env::set_var("MUGUET_DATA", config.game_root.join("data"));
                    std::env::set_var("MUGUET_SHADERS", build.join("shaders"));
                    std::env::set_var("MUGUET_START_MODE", "2");
                    crate::platform::raise_open_file_limit();
                    crate::input::set_size(config.width as i32, config.height as i32);
                    let engine = wasmtime::Engine::new(&crate::code::engine_config()?)?;
                    let module = crate::code::load_verified(
                        &engine,
                        &config.code_path,
                        &config.expected_code_sha256,
                    )?;
                    phase(Phase::ModuleReady);
                    let mut mem = None;
                    let mut imports = Vec::new();
                    for imp in module.imports() {
                        match imp.ty() {
                            wasmtime::ExternType::Memory(mt)
                                if imp.module() == "env" && imp.name() == "memory" =>
                            {
                                if mem.is_some() {
                                    anyhow::bail!("multiple env.memory imports");
                                }
                                mem = Some(wasmtime::SharedMemory::new(
                                    &engine,
                                    crate::code::heap_type(mt)?,
                                )?);
                            }
                            wasmtime::ExternType::Func(ty) => imports.push(crate::ImportSpec {
                                module: imp.module().into(),
                                name: imp.name().into(),
                                ty,
                            }),
                            other => anyhow::bail!(
                                "unsupported import {}.{}: {other:?}",
                                imp.module(),
                                imp.name()
                            ),
                        }
                    }
                    let counts = imports.iter().map(|_| AtomicU64::new(0)).collect();
                    let rt = Arc::new(crate::Runtime {
                        engine,
                        module,
                        mem: mem.context("module must import env.memory")?,
                        imports,
                        counts,
                        start: std::time::Instant::now(),
                    });
                    crate::RT
                        .set(rt.clone())
                        .map_err(|_| anyhow::anyhow!("runtime already set"))?;
                    *crate::gpu::SURFACE.lock().unwrap() = Some(surface);
                    phase(Phase::Running);
                    let mut args: Vec<String> = [
                        "./this.program",
                        "-rootdir=/game/",
                        "-nodisplaycalibration",
                        "-noSocialClub",
                        "-pc:nosocialclub",
                        "-nonetwork",
                        "-output",
                        "-forceResolution",
                        "-Script_all=warning",
                        "-replay_all=warning",
                        "-fastscriptcore",
                        "-frameLimit=2",
                        "-wasmstartchoice=2",
                        "-wgpu",
                    ]
                    .iter()
                    .map(|s| (*s).into())
                    .collect();
                    args.extend([
                        format!("-width={}", config.width),
                        format!("-height={}", config.height),
                    ]);
                    crate::run_engine(&rt, args)?;
                    anyhow::bail!("engine main returned");
                };
                if let Err(e) = run() {
                    fail(format!("{e:#}"));
                }
            })
        });
    if let Err(e) = spawn {
        fail(format!("spawn engine-main: {e}"));
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panic_failure_remains_visible_after_later_progress() {
        guard("fixture", || panic!("reported panic"));
        phase(Phase::Running);
        let s = status();
        assert_eq!(s["phase"], "failed");
        assert!(s["error"].as_str().unwrap().contains("reported panic"));
        assert!(s["gpuErrors"].is_u64());
        assert!(s["heapBytes"].is_u64());
    }
    #[test]
    fn only_one_session_is_accepted_even_after_failure() {
        let gate = SessionGate(AtomicBool::new(false));
        gate.accept().unwrap();
        assert!(gate
            .accept()
            .unwrap_err()
            .to_string()
            .contains("one game session"));
    }
}

/// Each callback case needs a fresh process because real session failure is sticky.
#[cfg(test)]
pub(crate) mod test_support {
    pub fn isolated_cases(test_name: &str, cases: &[&str], run: impl FnOnce(&str)) {
        const CASE_ENV: &str = "MUGUET_TASK1_FIX_CALLBACK_CASE";
        if let Ok(case) = std::env::var(CASE_ENV) {
            run(&case);
            return;
        }
        for case in cases {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", test_name, "--nocapture"])
                .env(CASE_ENV, case)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "callback case {case} failed: status={}\nstdout={}\nstderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
