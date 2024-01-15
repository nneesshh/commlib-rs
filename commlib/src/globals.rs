use parking_lot::{Condvar, Mutex};
use std::sync::Arc;

lazy_static::lazy_static! {
    ///
    pub static ref G_EXIT_CV: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));

    ///
    pub static ref G_THREAD_POOL: Arc<crate::utils::ThreadPool> = {
        //
        let pool = crate::utils::ThreadPoolBuilder::new().num_threads(4).build();
        Arc::new(pool)
    };
}
