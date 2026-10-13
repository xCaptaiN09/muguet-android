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
    serde_json::json!({"phase":s.0.as_str(),"error":s.1,"presents":crate::gpu::PRESENTS.load(Ordering::Relaxed),"gpuErrors":GPU_ERRORS.load(Ordering::Relaxed),"inVehicle":crate::pad::IN_VEHICLE.load(Ordering::Relaxed),"requestedGraphicsProfile":"mobile-conservative-v1","heapBytes":crate::RT.get().map(|rt|rt.mem.data_size()).unwrap_or(0)})
}
pub fn suspend(paused: bool) {
    crate::gpu::SUSPENDED.store(paused, Ordering::Relaxed);
    crate::input::set_focus(!paused);
    if paused {
        crate::input::set_locked(false);
    }
}
pub fn key(code: &str, pressed: bool) {
    if !pressed || crate::pad::FOCUSED.load(Ordering::Relaxed) {
        crate::input::key(code, pressed, false, None, false);
    }
}
pub fn mouse_button(dom_button: usize, pressed: bool) {
    if matches!(dom_button, 0 | 2) && (!pressed || crate::pad::FOCUSED.load(Ordering::Relaxed)) {
        crate::input::button(dom_button, pressed);
    }
}
pub fn touch_horn(pressed: bool) {
    crate::pad::HORN_TOUCH.store(
        pressed && crate::pad::FOCUSED.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
}
pub fn mouse_delta(dx: f64, dy: f64) {
    if crate::pad::FOCUSED.load(Ordering::Relaxed) && dx.is_finite() && dy.is_finite() {
        crate::input::set_locked(true);
        crate::input::mouse_delta_f(dx, dy);
    }
}
/// Requested retry preset; actual guest settings and memory use require device evidence.
fn session_arguments(width: u32, height: u32) -> Vec<String> {
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
        "-textureQuality=0",
        "-shaderQuality=1",
        "-grassQuality=0",
        "-reflectionQuality=0",
        "-shadowQuality=1",
        "-lodScale=0.35",
        "-particleQuality=0",
        "-waterQuality=0",
    ]
    .iter()
    .map(|s| (*s).into())
    .collect();
    args.extend([format!("-width={}", width), format!("-height={}", height)]);
    args
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
                    let args = session_arguments(config.width, config.height);
                    eprintln!(
                        "[session] requested conservative mobile graphics profile: {}",
                        args.join(" ")
                    );
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
    fn mobile_profile_keeps_launch_contract_and_safe_quality_ranges() {
        let args = session_arguments(960, 432);
        for flag in [
            "-width=960",
            "-height=432",
            "-frameLimit=2",
            "-wasmstartchoice=2",
            "-textureQuality=0",
            "-shaderQuality=1",
            "-grassQuality=0",
            "-reflectionQuality=0",
            "-shadowQuality=1",
            "-lodScale=0.35",
            "-particleQuality=0",
            "-waterQuality=0",
        ] {
            assert!(args.iter().any(|a| a == flag), "missing {flag}");
        }
        assert!(!args
            .iter()
            .any(|a| a == "-shaderQuality=0" || a == "-shadowQuality=0"));
    }
    #[test]
    fn touch_guest_input_and_suspend_clear_real_block() {
        test_support::isolated_cases(
            "session::tests::touch_guest_input_and_suspend_clear_real_block",
            &["input"],
            |_| {
                let mut cfg = wasmtime::Config::new();
                cfg.wasm_memory64(true)
                    .wasm_threads(true)
                    .shared_memory(true);
                let engine = wasmtime::Engine::new(&cfg).unwrap();
                let module =
                    wasmtime::Module::new(&engine, wat::parse_str("(module)").unwrap()).unwrap();
                let mem = wasmtime::SharedMemory::new(
                    &engine,
                    wasmtime::MemoryTypeBuilder::new()
                        .min(1)
                        .max(Some(1))
                        .memory64(true)
                        .shared(true)
                        .build()
                        .unwrap(),
                )
                .unwrap();
                let rt = Arc::new(crate::Runtime {
                    engine,
                    module,
                    mem,
                    imports: vec![],
                    counts: vec![],
                    start: std::time::Instant::now(),
                });
                assert!(crate::RT.set(rt.clone()).is_ok());
                crate::input::publish(1024);
                let read = |offset: usize| unsafe {
                    (&*(crate::mem_base_len(&rt.mem).0.add(1024 + offset)
                        as *const std::sync::atomic::AtomicI32))
                        .load(Ordering::SeqCst)
                };
                mouse_button(0, true);
                mouse_button(2, true);
                assert_eq!(read(276), 3);
                mouse_button(0, false);
                assert_eq!(read(276), 2);
                mouse_button(9, true);
                assert_eq!(read(276), 2);
                key("KeyW", true);
                touch_horn(true);
                assert!(crate::pad::horn_held());
                crate::pad::IN_VEHICLE.store(true, Ordering::Relaxed);
                assert_eq!(status()["inVehicle"], true);
                suspend(true);
                assert_eq!(read(276), 0);
                assert_eq!(crate::read_bytes(&rt.mem, 1024 + 0x57, 1), vec![0]);
                assert!(!crate::pad::horn_held());
                mouse_button(0, true);
                touch_horn(true);
                key("KeyW", true);
                assert_eq!(read(276), 0);
                assert!(!crate::pad::horn_held());
                assert_eq!(crate::read_bytes(&rt.mem, 1024 + 0x57, 1), vec![0]);
                suspend(false);
                mouse_button(0, true);
                assert_eq!(read(276), 1);
                suspend(true);
            },
        );
    }
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
