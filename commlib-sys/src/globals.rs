use hashbrown::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

#[allow(dead_code)]
static INIT: AtomicBool = AtomicBool::new(false);
#[allow(dead_code)]
static INIT_LOCK: Mutex<()> = Mutex::new(());

#[allow(dead_code)]
const SERVICE_ID_SIG: u32 = 1001_u32;

lazy_static::lazy_static! {
    pub static ref MAP_EMPTY: HashMap<char, u32> = HashMap::new();
    pub static ref SET_EMPTY: HashSet<char> = HashSet::new();
    pub static ref SET_VEC_EMPTY: Vec<char> = vec![];

}

lazy_static::lazy_static! {
    pub static ref G_SRV_SIGNAL: std::sync::RwLock<crate::ServiceSignalRs> =  std::sync::RwLock::new(crate::ServiceSignalRs::new(0));
    pub static ref G_EXIT: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)> = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
}

