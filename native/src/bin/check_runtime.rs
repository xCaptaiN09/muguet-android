fn main() {
    let report = muguet_android::runtime::run_runtime_probe();
    println!("{report}");
    if report["status"] != "passed" {
        std::process::exit(1);
    }
}
