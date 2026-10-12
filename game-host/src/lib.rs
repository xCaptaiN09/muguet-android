// muguet - reusable runtime adapted from main.rs
// Constantin Clerc (c22dev) - 0.1
// Android adaptation: Muguet Android contributors, GPL-3.0-or-later.
#![allow(dead_code)] // Imported host includes capabilities not exercised by the first boot UI.
mod audio;
pub mod code;
mod config;
mod fs;
mod gpu;
mod host;
mod input;
mod io;
mod mp;
mod pad;
#[cfg(feature = "jit")]
mod patch;
pub mod platform;
mod proto;
mod server;
mod session;
use anyhow::{Context, Result};
use platform::{engine_thread_qos, set_interactive_qos, set_qos};
pub use session::{key, launch, mouse_delta, status, suspend, LaunchConfig, SurfaceHandoff};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use wasmtime::*;
pub fn footprint_mb() -> u64 {
    platform::footprint_mb()
}
pub struct ImportSpec {
    pub module: String,
    pub name: String,
    pub ty: FuncType,
}

pub struct Runtime {
    pub engine: Engine,
    pub module: Module,
    pub mem: SharedMemory,
    pub imports: Vec<ImportSpec>,
    pub counts: Vec<AtomicU64>,
    pub start: Instant,
}

pub struct HostState {
    pub pthread: u64,
}

pub static RT: OnceLock<Arc<Runtime>> = OnceLock::new();

pub fn run_engine(rt: &Arc<Runtime>, args: Vec<String>) -> Result<()> {
    set_interactive_qos();
    let (mut store, instance) = instantiate(rt, 0)?;
    call_export(&mut store, &instance, "_emscripten_tls_init", &[])?;
    call_export(&mut store, &instance, "__wasm_call_ctors", &[])?;
    let argv = build_argv(&mut store, &instance, &rt.mem, &args)?;
    eprintln!("[host] main: {}", args.join(" "));
    let main = instance
        .get_func(&mut store, "__main_argc_argv")
        .context("no main")?;
    let mut out = [Val::I32(0)];
    match main.call(
        &mut store,
        &[Val::I32(args.len() as i32), Val::I64(argv as i64)],
        &mut out,
    ) {
        Ok(()) => eprintln!("[host] main returned {:?}", out[0]),
        Err(_) if host::take_unwind() => {
            eprintln!("[host] main unwound with a live runtime; waiting on engine threads");
            loop {
                std::thread::park();
            }
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

pub fn instantiate(rt: &Arc<Runtime>, pthread: u64) -> Result<(Store<HostState>, Instance)> {
    let mut store = Store::new(&rt.engine, HostState { pthread });
    let mut linker: Linker<HostState> = Linker::new(&rt.engine);
    linker.define(&store, "env", "memory", rt.mem.clone())?;
    for (idx, imp) in rt.imports.iter().enumerate() {
        linker.func_new(
            &imp.module,
            &imp.name,
            imp.ty.clone(),
            move |caller, params, results| {
                host::call(caller, idx, params, results).map_err(wasmtime::Error::from_anyhow)
            },
        )?;
    }
    let instance = linker.instantiate(&mut store, &rt.module)?;
    Ok((store, instance))
}

pub fn spawn_pthread(pthread_ptr: u64, start_routine: u64, arg: u64) -> i32 {
    let rt = RT.get().expect("runtime").clone();

    let name = if arg != 0 {
        read_cstr(&rt.mem, arg + 24)
    } else {
        String::new()
    };
    let name = if name.is_empty() || name.len() > 40 {
        format!("pthread-{pthread_ptr:x}")
    } else {
        name
    };
    let qos = engine_thread_qos(&name);
    let spawned = std::thread::Builder::new()
        .name(name)
        .stack_size(16 << 20)
        .spawn(move || {
            session::guard("pthread", || {
                set_qos(qos);
                if let Err(e) = run_pthread(&rt, pthread_ptr, start_routine, arg) {
                    session::fail(format!("pthread 0x{pthread_ptr:x} failed: {e:?}"));
                }
            });
        });
    if spawned.is_ok() {
        0
    } else {
        6
    }
}

fn run_pthread(rt: &Arc<Runtime>, ptr: u64, start_routine: u64, arg: u64) -> Result<()> {
    let (mut store, inst) = instantiate(rt, ptr)?;

    let high = read_u64(&rt.mem, ptr + 80).unwrap_or(0);
    let size = read_u64(&rt.mem, ptr + 88).unwrap_or(0);
    call_export(
        &mut store,
        &inst,
        "emscripten_stack_set_limits",
        &[Val::I64(high as i64), Val::I64((high - size) as i64)],
    )?;
    call_export(
        &mut store,
        &inst,
        "_emscripten_stack_restore",
        &[Val::I64(high as i64)],
    )?;

    call_export(
        &mut store,
        &inst,
        "_emscripten_thread_init",
        &[
            Val::I64(ptr as i64),
            Val::I32(0),
            Val::I32(0),
            Val::I32(1),
            Val::I64(0),
            Val::I32(0),
        ],
    )?;
    call_export(&mut store, &inst, "_emscripten_tls_init", &[])?;
    host::mailbox_await(&rt.mem, ptr);

    let table = inst
        .get_table(&mut store, "__indirect_function_table")
        .context("no function table")?;
    let entry = table
        .get(&mut store, start_routine)
        .and_then(|r| r.as_func().flatten().cloned())
        .with_context(|| format!("null start routine {start_routine}"))?;
    let mut res = [Val::I64(0)];
    let result = match entry.call(&mut store, &[Val::I64(arg as i64)], &mut res) {
        Ok(()) => match res[0] {
            Val::I64(v) => v,
            Val::I32(v) => v as i64,
            _ => 0,
        },
        Err(_) if host::take_unwind() => return Ok(()),
        Err(e) => {
            let name = read_thread_name(&rt.mem, ptr);
            return Err(anyhow::anyhow!("thread {name} trapped: {e:?}"));
        }
    };
    match call_export(
        &mut store,
        &inst,
        "_emscripten_thread_exit",
        &[Val::I64(result)],
    ) {
        Err(e) if !host::take_unwind() => Err(e),
        _ => Ok(()),
    }
}

fn read_thread_name(_mem: &SharedMemory, ptr: u64) -> String {
    format!("0x{ptr:x}")
}

pub fn call_export(
    store: &mut Store<HostState>,
    inst: &Instance,
    name: &str,
    args: &[Val],
) -> Result<Vec<Val>> {
    let f = inst
        .get_func(&mut *store, name)
        .with_context(|| format!("no export {name}"))?;
    let ty = f.ty(&mut *store);
    let args = coerce_args(args, &ty);
    let mut results: Vec<Val> = ty.results().map(|t| default_val(&t)).collect();
    f.call(&mut *store, &args, &mut results)?;
    Ok(results)
}

pub fn coerce_args(args: &[Val], ty: &FuncType) -> Vec<Val> {
    ty.params()
        .zip(args)
        .map(|(pt, v)| {
            let i = match v {
                Val::I32(x) => *x as i64,
                Val::I64(x) => *x,
                other => return *other,
            };
            match pt {
                ValType::I32 => Val::I32(i as i32),
                ValType::I64 => Val::I64(i),
                _ => *v,
            }
        })
        .collect()
}

fn build_argv(
    store: &mut Store<HostState>,
    inst: &Instance,
    mem: &SharedMemory,
    args: &[String],
) -> Result<u64> {
    let alloc = inst.get_typed_func::<i64, i64>(&mut *store, "_emscripten_stack_alloc")?;
    let mut ptrs = Vec::new();
    for a in args {
        let bytes = [a.as_bytes(), &[0]].concat();
        let p = alloc.call(&mut *store, bytes.len() as i64)? as u64;
        write_bytes(mem, p, &bytes);
        ptrs.push(p);
    }
    let argv = alloc.call(&mut *store, ((ptrs.len() + 1) * 8) as i64)? as u64;
    for (i, p) in ptrs.iter().chain(std::iter::once(&0)).enumerate() {
        write_u64(mem, argv + i as u64 * 8, *p);
    }
    Ok(argv)
}

pub fn default_val(t: &ValType) -> Val {
    match t {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::V128 => Val::V128(0u128.into()),
        ValType::Ref(_) => Val::null_any_ref(),
    }
}

pub fn mem_base_len(mem: &SharedMemory) -> (*mut u8, usize) {
    let d = mem.data();
    (d.as_ptr() as *mut u8, d.len())
}
pub fn read_u64(mem: &SharedMemory, addr: u64) -> Option<u64> {
    let (base, len) = mem_base_len(mem);
    let a = addr as usize;
    if a.checked_add(8).is_none_or(|end| end > len) {
        return None;
    }
    unsafe { Some(std::ptr::read_unaligned(base.add(a) as *const u64)) }
}
pub fn read_u32(mem: &SharedMemory, addr: u64) -> Option<u32> {
    let (base, len) = mem_base_len(mem);
    let a = addr as usize;
    if a.checked_add(4).is_none_or(|end| end > len) {
        return None;
    }
    unsafe { Some(std::ptr::read_unaligned(base.add(a) as *const u32)) }
}
pub fn write_u64(mem: &SharedMemory, addr: u64, v: u64) -> bool {
    write_bytes(mem, addr, &v.to_le_bytes())
}
pub fn write_u32(mem: &SharedMemory, addr: u64, v: u32) -> bool {
    write_bytes(mem, addr, &v.to_le_bytes())
}
pub fn read_bytes(mem: &SharedMemory, addr: u64, n: u64) -> Vec<u8> {
    let (base, len) = mem_base_len(mem);
    let (a, n) = (addr as usize, n as usize);
    if a.checked_add(n).is_none_or(|end| end > len) {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(base.add(a), n).to_vec() }
}
pub fn write_bytes(mem: &SharedMemory, addr: u64, data: &[u8]) -> bool {
    let (base, len) = mem_base_len(mem);
    let a = addr as usize;
    if a.checked_add(data.len()).is_none_or(|end| end > len) {
        return false;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(a), data.len());
    }
    true
}
pub fn read_cstr(mem: &SharedMemory, addr: u64) -> String {
    let (base, len) = mem_base_len(mem);
    let mut a = addr as usize;
    let mut out = Vec::new();
    while a < len && out.len() < 65536 {
        let b = unsafe { *base.add(a) };
        if b == 0 {
            break;
        }
        out.push(b);
        a += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
