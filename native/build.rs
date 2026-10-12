fn main() {
    println!("cargo:rerun-if-env-changed=MUGUET_ANDROID_CODE_SHA256");
    if let Ok(digest) = std::env::var("MUGUET_ANDROID_CODE_SHA256") {
        println!("cargo:rustc-env=MUGUET_ANDROID_CODE_SHA256={digest}");
    }
}
