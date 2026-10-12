use serde_json::{json, Value};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use wasmtime::{
    error::Context, Config, Engine, Instance, MemoryType, Module, Result, SharedMemory, Store,
};

const GUEST: &str = r#"
(module
  (import "env" "memory" (memory i64 1 2 shared))
  (func (export "write") (param i64 i32)
    local.get 0 local.get 1 i32.atomic.store)
  (func (export "read") (param i64) (result i32)
    local.get 0 i32.atomic.load)
  (func (export "wait") (param i64 i32 i64) (result i32)
    local.get 0 local.get 1 local.get 2 memory.atomic.wait32)
  (func (export "notify") (param i64 i32) (result i32)
    local.get 0 local.get 1 memory.atomic.notify))
"#;

const INITIAL_VALUE: i32 = 0x1234_5678;
const WORKER_VALUE: i32 = 0x2345_6789;
const WAIT_TIMEOUT_NS: i64 = 3_000_000_000;

/// Run a synthetic memory64/threads check, reporting only observed results.
pub fn run_runtime_probe() -> Value {
    let started = Instant::now();
    let mut checks = Vec::new();
    if let Err(error) = probe(&mut checks) {
        checks.push(json!({
            "name": "runtime_error", "status": "failed",
            "details": { "error": format!("{error:#}") }
        }));
    }
    let passed = checks.iter().all(|check| check["status"] == "passed");
    json!({
        "status": if passed { "passed" } else { "failed" },
        "checks": checks,
        "elapsed_ms": started.elapsed().as_millis() as u64
    })
}

fn record(checks: &mut Vec<Value>, name: &str, passed: bool, details: Value) {
    checks.push(json!({
        "name": name,
        "status": if passed { "passed" } else { "failed" },
        "details": details
    }));
}

fn probe(checks: &mut Vec<Value>) -> Result<()> {
    let mut config = Config::new();
    config
        .wasm_memory64(true)
        .wasm_threads(true)
        .shared_memory(true);
    let engine = Engine::new(&config).context("create memory64/threads engine")?;
    let module = Module::new(&engine, GUEST).context("compile shared memory64 guest")?;
    let memory_type = MemoryType::builder()
        .memory64(true)
        .shared(true)
        .min(1)
        .max(Some(2))
        .build()?;
    let memory = SharedMemory::new(&engine, memory_type).context("allocate shared memory")?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[memory.clone().into()])?;
    let write = instance.get_typed_func::<(i64, i32), ()>(&mut store, "write")?;
    let read = instance.get_typed_func::<i64, i32>(&mut store, "read")?;
    let notify = instance.get_typed_func::<(i64, i32), i32>(&mut store, "notify")?;

    write
        .call(&mut store, (0, INITIAL_VALUE))
        .context("guest atomic write")?;
    let observed = read.call(&mut store, 0).context("guest atomic read")?;
    record(
        checks,
        "guest_atomics",
        observed == INITIAL_VALUE,
        json!({
            "address": 0, "written": INITIAL_VALUE, "observed": observed
        }),
    );

    let initial_pages = memory.size();
    let previous_pages = memory.grow(1).context("grow to two pages")?;
    let grown_pages = memory.size();
    let grown_bytes = memory.data_size();
    let rejection = memory.grow(1);
    let rejected = rejection.is_err();
    let rejection_error = rejection.err().map(|error| format!("{error:#}"));
    let final_pages = memory.size();
    record(
        checks,
        "memory_growth",
        initial_pages == 1
            && previous_pages == 1
            && grown_pages == 2
            && grown_bytes == 131_072
            && rejected
            && final_pages == 2,
        json!({
            "initial_pages": initial_pages, "grow_previous_pages": previous_pages,
            "grown_pages": grown_pages, "grown_bytes": grown_bytes,
            "out_of_bound_growth_rejected": rejected, "rejection_error": rejection_error,
            "final_pages": final_pages
        }),
    );

    let worker_engine = engine.clone();
    let worker_memory = memory.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    // A separate store/instance imports the same SharedMemory. The worker owns
    // everything it uses, so an error/timeout can safely return without joining
    // an unresponsive thread on the app's diagnostic worker.
    let _worker = thread::Builder::new()
        .name("wasm-wait-probe".into())
        .spawn(move || {
            let result = (|| -> Result<(i32, i32)> {
                let mut store = Store::new(&worker_engine, ());
                let instance = Instance::new(&mut store, &module, &[worker_memory.into()])?;
                let read = instance.get_typed_func::<i64, i32>(&mut store, "read")?;
                let write = instance.get_typed_func::<(i64, i32), ()>(&mut store, "write")?;
                let wait = instance.get_typed_func::<(i64, i32, i64), i32>(&mut store, "wait")?;
                let worker_observed = read.call(&mut store, 0)?;
                ready_tx.send(()).context("signal waiter readiness")?;
                let wait_result = wait.call(&mut store, (0, INITIAL_VALUE, WAIT_TIMEOUT_NS))?;
                write.call(&mut store, (4, WORKER_VALUE))?;
                Ok((wait_result, worker_observed))
            })();
            let _ = done_tx.send(result);
        })
        .context("spawn Wasm waiter")?;

    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .context("waiter readiness timeout")?;
    let notify_started = Instant::now();
    let mut notified_waiters = 0;
    let mut notify_attempts = 0;
    // Readiness precedes the guest wait; retrying closes that lost-wakeup gap.
    // Do not change the waited-on value: a successful wait must return 0, not
    // the "not equal" result (1), and must have an observed notification.
    while notified_waiters == 0 && notify_started.elapsed() < Duration::from_secs(2) {
        notified_waiters = notify.call(&mut store, (0, 1)).context("guest notify")?;
        notify_attempts += 1;
        if notified_waiters == 0 {
            thread::sleep(Duration::from_millis(1));
        }
    }
    let (wait_result, worker_observed) = done_rx
        .recv_timeout(Duration::from_secs(4))
        .context("waiter completion timeout")?
        .context("worker guest operations")?;
    let main_observed = read
        .call(&mut store, 4)
        .context("read worker atomic write")?;
    record(
        checks,
        "thread_wait_notify",
        notified_waiters == 1
            && wait_result == 0
            && worker_observed == INITIAL_VALUE
            && main_observed == WORKER_VALUE,
        json!({
            "wait_result": wait_result, "notified_waiters": notified_waiters,
            "notify_attempts": notify_attempts, "worker_observed": worker_observed,
            "worker_written": WORKER_VALUE, "main_observed": main_observed,
            "wait_timeout_ns": WAIT_TIMEOUT_NS,
            "notify_elapsed_ms": notify_started.elapsed().as_millis() as u64
        }),
    );
    Ok(())
}
