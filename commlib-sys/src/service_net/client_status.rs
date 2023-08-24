///
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum ClientStatus {
    Null = 0,
    Initializing = 1,
    Initialized = 2,
    Connecting = 3,
    Connected = 4,
    Disconnecting = 5,
    Disconnected = 6,
}

impl ClientStatus {
    ///
    pub fn to_string(&self) -> &'static str {
        match self {
            ClientStatus::Null => "kNull",
            ClientStatus::Initializing => "kInitializing",
            ClientStatus::Initialized => "kInitialized",
            ClientStatus::Connecting => "kConnecting",
            ClientStatus::Connected => "kConnected",
            ClientStatus::Disconnecting => "kDisconnecting",
            ClientStatus::Disconnected => "kDisconnected",
        }
    }

    ///
    pub fn is_connected(&self) -> bool {
        match self {
            ClientStatus::Connected => true,
            _ => false,
        }
    }
}
