use parking_lot::{Mutex, RwLock};
use std::sync::{atomic::AtomicBool, Arc};

#[allow(dead_code)]
static INIT: AtomicBool = AtomicBool::new(false);
#[allow(dead_code)]
static INIT_LOCK: Mutex<()> = Mutex::new(());

lazy_static::lazy_static! {
    pub static ref G_CONF: Arc<RwLock<crate::conf::Conf>> = Arc::new(RwLock::new(crate::conf::Conf::new()));
}
