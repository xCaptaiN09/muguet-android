//! Isolated game-process JNI boundary. No Java local reference crosses a thread.
use crate::android::NativeWindow;
use anyhow::{ensure, Context, Result};
use jni::{
    objects::{GlobalRef, JClass, JObject, JString},
    sys::{jboolean, jfloat, jint, jstring},
    JNIEnv,
};
use muguet_engine::{LaunchConfig, SurfaceHandoff};
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    os::fd::AsRawFd,
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    ptr,
    sync::{Arc, Mutex, OnceLock},
};

static APP_CONTEXT: OnceLock<GlobalRef> = OnceLock::new();
static START_ERROR: Mutex<Option<String>> = Mutex::new(None);

fn trusted_digest() -> Result<&'static str> {
    let digest = option_env!("MUGUET_ANDROID_CODE_SHA256")
        .context("APK has no trusted game-code SHA-256 pin")?;
    ensure!(
        digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
        "APK game-code SHA-256 pin is invalid"
    );
    Ok(digest)
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_verifyGameCode(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    path: JString<'_>,
) -> jboolean {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        let expected = trusted_digest()?;
        let path = string(&mut env, path)?;
        let bytes = std::fs::read(path)?;
        muguet_engine::code::verify_bytes(&bytes, expected)
    }));
    u8::from(matches!(result, Ok(Ok(()))))
}

fn string(env: &mut JNIEnv<'_>, value: JString<'_>) -> Result<String> {
    Ok(env.get_string(&value)?.into())
}

fn json_string(env: &mut JNIEnv<'_>, value: Value) -> jstring {
    match env.new_string(value.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

fn failed(message: String) -> Value {
    json!({"phase":"failed","error":message})
}

fn boundary(env: &mut JNIEnv<'_>, f: impl FnOnce(&mut JNIEnv<'_>) -> Result<Value>) -> jstring {
    let value = match catch_unwind(AssertUnwindSafe(|| f(env))) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => failed(format!("{error:#}")),
        Err(_) => failed("Caught Rust panic in game JNI boundary".into()),
    };
    if value["phase"] == "failed" {
        *START_ERROR.lock().unwrap_or_else(|e| e.into_inner()) =
            value["error"].as_str().map(str::to_owned);
    }
    json_string(env, value)
}

fn initialize(env: &mut JNIEnv<'_>, context: JObject<'_>, log_path: JString<'_>) -> Result<Value> {
    ensure!(!context.is_null(), "application Context is null");
    if APP_CONTEXT.get().is_none() {
        let log_path = string(env, log_path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("open game log {log_path}"))?;
        // This is the isolated :game process. All Rust engine stderr, including
        // thread failures, is retained in private storage for app-side export.
        let result = unsafe { libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO) };
        ensure!(result >= 0, "redirect game stderr failed");
        let vm = env.get_java_vm()?;
        let global = env.new_global_ref(context)?;
        let raw = global.as_raw();
        APP_CONTEXT
            .set(global)
            .map_err(|_| anyhow::anyhow!("Android context was already initialized"))?;
        // SAFETY: both JavaVM and APP_CONTEXT's GlobalRef remain live for the
        // entire :game process, including all AAudio worker threads.
        unsafe {
            ndk_context::initialize_android_context(vm.get_java_vm_pointer().cast(), raw.cast());
        }
        eprintln!("[game-host] native logging initialized");
    }
    Ok(json!({"phase":"ready"}))
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_initializeGame(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    context: JObject<'_>,
    log_path: JString<'_>,
) -> jstring {
    boundary(&mut env, |env| initialize(env, context, log_path))
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_startGame(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    surface: JObject<'_>,
    game_root: JString<'_>,
    code_path: JString<'_>,
    saves_dir: JString<'_>,
    cache_dir: JString<'_>,
    width: jint,
    height: jint,
) -> jstring {
    boundary(&mut env, |env| {
        let digest = trusted_digest()?.to_string();
        ensure!(
            APP_CONTEXT.get().is_some(),
            "Android game context was not initialized"
        );
        ensure!(
            width > 0 && height > 0,
            "invalid game dimensions {width}x{height}"
        );
        let config = LaunchConfig {
            game_root: PathBuf::from(string(env, game_root)?),
            code_path: PathBuf::from(string(env, code_path)?),
            expected_code_sha256: digest,
            saves_dir: PathBuf::from(string(env, saves_dir)?),
            cache_dir: PathBuf::from(string(env, cache_dir)?),
            width: width as u32,
            height: height as u32,
        };
        let window = Arc::new(NativeWindow::acquire(env, &surface)?);
        let instance = crate::gpu::vulkan_instance();
        let game_surface = window.create_game_surface(&instance)?;
        let handoff = SurfaceHandoff {
            instance,
            surface: game_surface,
            owner: window,
        };
        muguet_engine::launch(config, handoff)?;
        Ok(json!({"phase":"accepted"}))
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_gameStatus(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    boundary(&mut env, |_| {
        let status = muguet_engine::status();
        let error = START_ERROR
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if status["phase"] != "failed" {
            if let Some(error) = error {
                return Ok(failed(error));
            }
        }
        Ok(status)
    })
}

fn void_boundary(name: &str, f: impl FnOnce() -> Result<()>) {
    let error = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => return,
        Ok(Err(error)) => format!("{name}: {error:#}"),
        Err(_) => format!("{name}: caught Rust panic"),
    };
    *START_ERROR.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_pauseGame(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    paused: jboolean,
) {
    void_boundary("pauseGame", || {
        muguet_engine::suspend(paused != 0);
        Ok(())
    });
}
#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_gameKey(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    code: JString<'_>,
    pressed: jboolean,
) {
    void_boundary("gameKey", || {
        let code = string(&mut env, code)?;
        muguet_engine::key(&code, pressed != 0);
        Ok(())
    });
}
#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_gameLook(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    dx: jfloat,
    dy: jfloat,
) {
    void_boundary("gameLook", || {
        muguet_engine::mouse_delta(dx as f64, dy as f64);
        Ok(())
    });
}

#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_gameMouseButton(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    button: jint,
    pressed: jboolean,
) {
    void_boundary("gameMouseButton", || {
        muguet_engine::mouse_button(button as usize, pressed != 0);
        Ok(())
    });
}
#[no_mangle]
pub extern "system" fn Java_dev_xcaptain_muguet_NativeBridge_gameTouchHorn(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    pressed: jboolean,
) {
    void_boundary("gameTouchHorn", || {
        muguet_engine::touch_horn(pressed != 0);
        Ok(())
    });
}
