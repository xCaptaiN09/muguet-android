// Desktop bundle resources are not used on Android.
pub fn bundled_resources() -> Option<std::path::PathBuf> {
    None
}
