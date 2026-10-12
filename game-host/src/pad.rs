// Android controls enter through the platform-independent input block.
use std::sync::atomic::AtomicBool;
pub static ACTIVE: AtomicBool = AtomicBool::new(false);
pub static FOCUSED: AtomicBool = AtomicBool::new(true);
pub static IN_VEHICLE: AtomicBool = AtomicBool::new(false);
pub fn horn_held() -> bool {
    false
}
pub fn release_all() {}
