// Android controls enter through the platform-independent input block.
use std::sync::atomic::{AtomicBool, Ordering};
pub static ACTIVE: AtomicBool = AtomicBool::new(false);
pub static FOCUSED: AtomicBool = AtomicBool::new(true);
pub static IN_VEHICLE: AtomicBool = AtomicBool::new(false);
pub static HORN_TOUCH: AtomicBool = AtomicBool::new(false);
pub fn horn_held() -> bool {
    HORN_TOUCH.load(Ordering::Relaxed)
}
pub fn release_all() {
    HORN_TOUCH.store(false, Ordering::Relaxed);
}
