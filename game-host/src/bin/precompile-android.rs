use anyhow::{Context, Result};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        anyhow::bail!("usage: precompile-android <game.wasm> <output.cwasm>");
    }
    let start = std::time::Instant::now();
    let wasm = std::fs::read(&args[1]).context("read game.wasm")?;
    let code = muguet_engine::code::precompile(&wasm)?;
    let digest = muguet_engine::code::digest(&code);
    std::fs::write(&args[2], &code)?;
    let mut hash_path = args[2].clone();
    hash_path.push(".sha256");
    std::fs::write(hash_path, format!("{digest}\n"))?;
    println!(
        "{}",
        serde_json::json!({"sha256":digest,"bytes":code.len(),"seconds":start.elapsed().as_secs_f64(),"target":muguet_engine::code::TARGET,"wasmtime":"49.0.2","heapMax":muguet_engine::code::HEAP_MAX,"reservation":muguet_engine::code::HEAP_MAX,"guard":muguet_engine::code::GUARD_BYTES,"memoryMayMove":false,"cpu":"generic Armv8-A","patchVersion":6})
    );
    Ok(())
}
