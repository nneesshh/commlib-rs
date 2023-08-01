use hashbrown::{HashMap, HashSet};
use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::{atomic::AtomicBool, Arc};

/// Service ID
pub const SERVICE_ID_SIG: u64 = 1001_u64;
pub const SERVICE_ID_NET: u64 = 1002_u64;
pub const SERVICE_ID_HTTP: u64 = 1003_u64;

lazy_static::lazy_static! {
    pub static ref G_SERVICE_SIGNAL: Arc<crate::ServiceSignalRs> =  Arc::new(crate::ServiceSignalRs::new(SERVICE_ID_SIG));
    pub static ref G_SERVICE_NET: Arc<crate::ServiceNetRs> =  Arc::new(crate::ServiceNetRs::new(SERVICE_ID_NET));

    pub static ref G_EXIT_CV: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
}
