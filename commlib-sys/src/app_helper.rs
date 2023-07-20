use crate::commlib_service::Service;

/// App: 应用框架
pub struct App {
    mtx: std::sync::Mutex<()>,
    services: Vec<Box<dyn Service>>,
    creators: hashbrown::HashMap<u64, fn(dyn Service)>,
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
