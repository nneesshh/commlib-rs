use parking_lot::Mutex;

use std::cell::UnsafeCell;
use std::sync::atomic::AtomicBool;

#[allow(dead_code)]
static INIT: AtomicBool = AtomicBool::new(false);
#[allow(dead_code)]
static INIT_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    pub static G_CONF: UnsafeCell<crate::conf::Conf> = UnsafeCell::new(crate::conf::Conf::new());
}
