///
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum ServerStatus {
    Null = 0,
    Initializing = 1,
    Initialized = 2,
    Starting = 3,
    Running = 4,
    Stopping = 5,
    Stopped = 6,
    Down = 7,
}

///
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum ServerSubStatus {
    SubStatusNull = 0,
    StoppingListener = 1,
    StoppingThreadPool = 2,
}

impl ServerStatus {
    ///
    pub fn to_string(&self) -> &'static str {
        match self {
            ServerStatus::Null => "kNull",
            ServerStatus::Initializing => "kInitializing",
            ServerStatus::Initialized => "kInitialized",
            ServerStatus::Starting => "kStarting",
            ServerStatus::Running => "kRunning",
            ServerStatus::Stopping => "kStopping",
            ServerStatus::Stopped => "kStopped",
            ServerStatus::Down => "kDown",
        }
    }

    ///
    pub fn is_running(&self) -> bool {
        match self {
            ServerStatus::Running => true,
            _ => false,
        }
    }

    ///
    pub fn is_stopped(&self) -> bool {
        match self {
            ServerStatus::Stopped => true,
            _ => false,
        }
    }

    ///
    pub fn is_stopping(&self) -> bool {
        match self {
            ServerStatus::Stopping => true,
            _ => false,
        }
    }
}

impl ServerSubStatus {
    ///
    pub fn to_string(&self) -> &'static str {
        match self {
            ServerSubStatus::SubStatusNull => "kSubStatusNull",
            ServerSubStatus::StoppingListener => "kStoppingListener",
            ServerSubStatus::StoppingThreadPool => "kStoppingThreadPool",
        }
    }
}
