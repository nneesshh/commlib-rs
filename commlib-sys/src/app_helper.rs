use crate::commlib_service::ServiceRs;

/// App: 应用框架
pub struct App {
    mtx: std::sync::Mutex<()>,
    services: Vec<crate::ServiceWrapper>,
    creators: hashbrown::HashMap<u64, fn(&crate::ServiceWrapper)>,
}

impl App {
    /// Constructor
    pub fn new() -> Self {
        Self {
            mtx: std::sync::Mutex::default(),
            services: Vec::default(),
            creators: hashbrown::HashMap::default(),
        }
    }

    pub fn init() {}
}
