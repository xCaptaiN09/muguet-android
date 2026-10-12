//! Identical Android AOT compiler/runtime configuration and trusted code loading.
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use wasmtime::{Config, Engine, MemoryType, MemoryTypeBuilder, Module};
pub const HEAP_MAX: u64 = 5 << 30;
pub const GUARD_BYTES: u64 = 64 << 10;
pub const TARGET: &str = "aarch64-linux-android";
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn verify_bytes(bytes: &[u8], expected: &str) -> Result<()> {
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("missing or invalid embedded SHA-256 digest");
    }
    if !digest(bytes).eq_ignore_ascii_case(expected) {
        bail!("compiled code SHA-256 mismatch");
    }
    Ok(())
}
pub fn engine_config() -> Result<Config> {
    let mut c = Config::new();
    c.target(TARGET)?;
    c.wasm_memory64(true)
        .wasm_threads(true)
        .shared_memory(true)
        .wasm_bulk_memory(true)
        .wasm_multi_memory(true)
        .memory_reservation(HEAP_MAX)
        .memory_guard_size(GUARD_BYTES)
        .memory_may_move(false);
    #[cfg(feature = "jit")]
    {
        c.cranelift_opt_level(wasmtime::OptLevel::Speed);
        // Explicit generic ISA prevents native CPU autodetection on either compiler or phone.
        // AArch64 defaults are baseline Armv8-A with optional extensions disabled.
    }
    Ok(c)
}
pub fn heap_type(mt: MemoryType) -> Result<MemoryType> {
    let cap = HEAP_MAX / (1u64 << mt.page_size_log2());
    if mt.minimum() > cap {
        bail!("imported memory minimum exceeds 5 GiB heap cap");
    }
    if !mt.is_shared() || !mt.is_64() {
        bail!("game requires shared memory64");
    }
    Ok(MemoryTypeBuilder::new()
        .min(mt.minimum())
        .max(Some(mt.maximum().unwrap_or(cap).min(cap)))
        .memory64(true)
        .shared(true)
        .page_size_log2(mt.page_size_log2())
        .build()?)
}
pub(crate) fn load_verified(
    engine: &Engine,
    path: &std::path::Path,
    expected: &str,
) -> Result<Module> {
    let bytes =
        std::fs::read(path).with_context(|| format!("read compiled code {}", path.display()))?;
    verify_bytes(&bytes, expected)?;
    // SAFETY: caller supplies an embedded digest of the artifact produced by our pinned
    // compiler, never a checksum read from user-writable game storage. Hash the exact
    // bytes deserialized to avoid a filesystem race between verification and loading.
    unsafe { Module::deserialize(engine, &bytes) }
        .map_err(anyhow::Error::from)
        .context("deserialize verified Android game code")
}
#[cfg(feature = "compiler")]
pub fn precompile(wasm: &[u8]) -> Result<Vec<u8>> {
    let patched = if crate::patch::is_patched(wasm) {
        wasm.to_vec()
    } else {
        crate::patch::patch(wasm, crate::mp::COMMANDS)?
    };
    Engine::new(&engine_config()?)?
        .precompile_module(&patched)
        .map_err(anyhow::Error::from)
        .context("compile patched Android module")
}
