use bytemuck::NoUninit;

#[derive(PartialEq, Copy, Clone, NoUninit)]
#[repr(u8)]
pub enum ClientStatus {
    Null = 0,
    Connecting = 1,
    Connected = 2,
    Disconnecting = 3,
    Disconnected = 4,
}

impl ClientStatus {
    ///
    pub fn to_string(&self) -> &'static str {
        match self {
            ClientStatus::Null => "kNull",
            ClientStatus::Connecting => "kConnecting",
            ClientStatus::Connected => "kConnected",
            ClientStatus::Disconnecting => "kDisconnecting",
            ClientStatus::Disconnected => "kDisconnected",
        }
    }

    /// 是否处于已连接状态
    #[inline(always)]
    pub fn is_connected(&self) -> bool {
        match self {
            ClientStatus::Connected => true,
            _ => false,
        }
    }

    /// 是否处于空闲状态
    #[inline(always)]
    pub fn is_idle(&self) -> bool {
        match self {
            ClientStatus::Null | ClientStatus::Disconnected => true,
            _ => false,
        }
    }
}
