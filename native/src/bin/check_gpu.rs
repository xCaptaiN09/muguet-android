fn main() {
    let report = muguet_android::gpu::probe_vulkan();
    println!("{report}");
    if report["status"] != "passed" {
        std::process::exit(1);
    }
}
