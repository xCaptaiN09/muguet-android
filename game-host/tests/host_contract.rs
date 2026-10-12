use muguet_engine::{code, platform};
#[test]
fn rejects_untrusted_compiled_code_before_deserialization() {
    let result = code::verify_bytes(b"not native code", &"0".repeat(64));
    assert!(result.unwrap_err().to_string().contains("SHA-256"));
}
#[test]
fn rejects_missing_content() {
    let root = tempfile::tempdir().unwrap();
    assert!(platform::discover_build(root.path())
        .unwrap_err()
        .to_string()
        .contains("manifest"));
}
#[test]
fn rejects_ambiguous_build_selection() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("data")).unwrap();
    std::fs::write(root.path().join("data/manifest.json"), r#"{"files":[]}"#).unwrap();
    for b in ["one", "two"] {
        let p = root.path().join("b").join(b).join("shaders");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("index.json"), "{}").unwrap();
    }
    assert!(platform::discover_build(root.path())
        .unwrap_err()
        .to_string()
        .contains("ambiguous"));
}
#[test]
fn rejects_invalid_dimensions() {
    for (w, h) in [(0, 432), (960, 0), (u32::MAX, 432), (960, u32::MAX)] {
        assert!(platform::validate_dimensions(w, h).is_err());
    }
}
#[test]
fn caps_shared_heap_at_five_gib() {
    let mt = wasmtime::MemoryTypeBuilder::new()
        .min(1)
        .max(Some(262144))
        .memory64(true)
        .shared(true)
        .build()
        .unwrap();
    assert_eq!(
        code::heap_type(mt).unwrap().maximum(),
        Some(5 * 1024 * 1024 * 1024 / 65536)
    );
}
#[test]
fn rejects_initial_heap_above_cap() {
    let mt = wasmtime::MemoryTypeBuilder::new()
        .min(100000)
        .max(Some(262144))
        .memory64(true)
        .shared(true)
        .build()
        .unwrap();
    assert!(code::heap_type(mt).is_err());
}
#[test]
fn accepts_digest_of_exact_bytes_and_rejects_missing_digest() {
    let bytes = b"arbitrary bytes are only tested for hashing here";
    let hash = code::digest(bytes);
    code::verify_bytes(bytes, &hash).unwrap();
    assert!(code::verify_bytes(bytes, "").is_err());
    assert!(code::verify_bytes(b"different", &hash).is_err());
}
#[test]
fn discovers_single_shader_build_without_requiring_game_wasm() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("data")).unwrap();
    std::fs::write(root.path().join("data/manifest.json"), r#"{"files":[]}"#).unwrap();
    let build = root.path().join("b/verified");
    std::fs::create_dir_all(build.join("shaders")).unwrap();
    std::fs::write(build.join("shaders/index.json"), "{}").unwrap();
    assert_eq!(platform::discover_build(root.path()).unwrap(), build);
    platform::validate_dimensions(960, 432).unwrap();
}
#[cfg(all(feature = "jit", target_arch = "x86_64", target_os = "linux"))]
#[test]
fn capped_shared_memory_is_valid_subtype_of_actual_game_import() {
    let mut config = code::engine_config().unwrap();
    config.target("x86_64-unknown-linux-gnu").unwrap();
    let engine = wasmtime::Engine::new(&config).unwrap();
    let wasm=wat::parse_str(r#"(module (import "env" "memory" (memory i64 49152 262144 shared)) (func (export "size") (result i64) memory.size))"#).unwrap();
    let module = wasmtime::Module::new(&engine, &wasm).unwrap();
    let mt = match module.imports().next().unwrap().ty() {
        wasmtime::ExternType::Memory(mt) => mt,
        _ => panic!("memory"),
    };
    let mem = wasmtime::SharedMemory::new(&engine, code::heap_type(mt).unwrap()).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let mut linker = wasmtime::Linker::new(&engine);
    linker.define(&store, "env", "memory", mem.clone()).unwrap();
    let inst = linker.instantiate(&mut store, &module).unwrap();
    assert_eq!(
        inst.get_typed_func::<(), i64>(&mut store, "size")
            .unwrap()
            .call(&mut store, ())
            .unwrap(),
        49152
    );
    assert!(mem.grow(40000).is_err());
    assert_eq!(mem.data_size(), 3usize << 30);
}

#[cfg(any(feature = "compiler", target_arch = "aarch64"))]
#[test]
fn compiler_and_runtime_use_android_memory_configuration() {
    let c = wasmtime::Engine::new(&code::engine_config().unwrap()).unwrap();
    assert_eq!(c.get_target().as_deref(), Some("aarch64-linux-android"));
    assert_eq!(c.get_memory_reservation(), 5 << 30);
    assert_eq!(c.get_memory_guard_size(), 64 << 10);
    assert!(!c.get_memory_may_move());
}
#[cfg(feature = "jit")]
#[test]
fn memory64_host_helpers_reject_wrapping_offsets() {
    let mut config = wasmtime::Config::new();
    config
        .wasm_threads(true)
        .wasm_memory64(true)
        .shared_memory(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
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
    assert_eq!(muguet_engine::read_u64(&mem, u64::MAX), None);
    assert_eq!(muguet_engine::read_u32(&mem, u64::MAX), None);
    assert!(!muguet_engine::write_bytes(&mem, u64::MAX, b"test"));
    assert!(muguet_engine::read_bytes(&mem, u64::MAX, 8).is_empty());
    assert!(muguet_engine::write_bytes(&mem, 65532, b"test"));
    assert_eq!(muguet_engine::read_bytes(&mem, 65532, 4), b"test");
}
#[cfg(all(feature = "jit", target_arch = "x86_64", target_os = "linux"))]
#[test]
fn guest_exit_is_reported_without_terminating_host() {
    const CHILD: &str = "MUGUET_TEST_GUEST_EXIT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "guest_exit_is_reported_without_terminating_host"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "guest terminated the host test process: {:?}, stderr={}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    let mut config = wasmtime::Config::new();
    config
        .wasm_memory64(true)
        .wasm_threads(true)
        .shared_memory(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(
        &engine,
        wat::parse_str(
            r#"(module
        (import "env" "memory" (memory i64 1 1 shared))
        (import "env" "proc_exit" (func $exit (param i32)))
        (func (export "exit") i32.const 7 call $exit))"#,
        )
        .unwrap(),
    )
    .unwrap();
    let mt = wasmtime::MemoryTypeBuilder::new()
        .min(1)
        .max(Some(1))
        .memory64(true)
        .shared(true)
        .build()
        .unwrap();
    let mem = wasmtime::SharedMemory::new(&engine, mt).unwrap();
    let imports = module
        .imports()
        .filter_map(|imp| match imp.ty() {
            wasmtime::ExternType::Func(ty) => Some(muguet_engine::ImportSpec {
                module: imp.module().into(),
                name: imp.name().into(),
                ty,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    let counts = imports
        .iter()
        .map(|_| std::sync::atomic::AtomicU64::new(0))
        .collect();
    let rt = std::sync::Arc::new(muguet_engine::Runtime {
        engine,
        module,
        mem,
        imports,
        counts,
        start: std::time::Instant::now(),
    });
    assert!(muguet_engine::RT.set(rt.clone()).is_ok());
    let (mut store, inst) = muguet_engine::instantiate(&rt, 0).unwrap();
    let err = muguet_engine::call_export(&mut store, &inst, "exit", &[]).unwrap_err();
    assert!(format!("{err:#}").contains("engine exit(7)"));
}
