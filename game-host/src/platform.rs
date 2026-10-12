//! Android platform boundaries; no desktop window or Apple scheduling dependencies.
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
pub fn validate_dimensions(w: u32, h: u32) -> Result<()> {
    if !(1..=8192).contains(&w) || !(1..=8192).contains(&h) {
        bail!("invalid render dimensions {w}x{h}");
    }
    Ok(())
}
pub fn discover_build(root: &Path) -> Result<PathBuf> {
    let manifest = root.join("data/manifest.json");
    let bytes = std::fs::read(&manifest).context("missing game data/manifest.json")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&bytes).context("invalid game manifest")?;
    if !manifest["files"].is_array() {
        bail!("game manifest is missing files array");
    }
    let mut builds: Vec<_> = std::fs::read_dir(root.join("b"))
        .context("missing game build directory b/")?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|e| e.path())
        .filter(|p| p.join("shaders/index.json").is_file())
        .collect();
    builds.sort();
    match builds.len() {
        0 => bail!("missing game build with shaders/index.json"),
        1 => Ok(builds.remove(0)),
        n => bail!("ambiguous game build selection: {n} shader builds"),
    }
}
pub(crate) fn raise_open_file_limit() {
    let mut r = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut r) == 0 {
            r.rlim_cur = r.rlim_cur.max(r.rlim_max.min(10240));
            libc::setrlimit(libc::RLIMIT_NOFILE, &r);
        }
    }
}
pub(crate) fn set_interactive_qos() {
    set_qos(0);
}
pub(crate) fn set_qos(_class: u32) {}
pub(crate) fn engine_thread_qos(_name: &str) -> u32 {
    0
}

/// Resident pages from Linux/Android procfs; no Apple process metrics.
pub(crate) fn footprint_mb() -> u64 {
    let pages = std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
        .unwrap_or(0);
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size > 0 {
        pages.saturating_mul(page_size as u64) >> 20
    } else {
        0
    }
}
