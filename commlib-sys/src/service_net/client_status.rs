///
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum ClientStatus {
    Null = 0,
    Initializing = 1,
    Initialized = 2,
    Starting = 3,
    Running = 4,
    Stopping = 5,
    Stopped = 6,
}

impl ClientStatus {
    ///
    pub fn to_string(&self) -> &'static str {
        match self {
            ClientStatus::Null => "kNull",
            ClientStatus::Initializing => "kInitializing",
            ClientStatus::Initialized => "kInitialized",
            ClientStatus::Starting => "kStarting",
            ClientStatus::Running => "kRunning",
            ClientStatus::Stopping => "kStopping",
            ClientStatus::Stopped => "kStopped",
        }
    }

    ///
    pub fn is_running(&self) -> bool {
        match self {
            ClientStatus::Running => true,
            _ => false,
        }
    }

    ///
    pub fn is_stopped(&self) -> bool {
        match self {
            ClientStatus::Stopped => true,
            _ => false,
        }
    }

    ///
    pub fn is_stopping(&self) -> bool {
        match self {
            ClientStatus::Stopping => true,
            _ => false,
        }
    }
}
