use bytemuck::NoUninit;

#[derive(PartialEq, Copy, Clone, NoUninit)]
#[repr(u8)]
pub enum ServerStatus {
    Null = 0,
    Starting = 1,
    Running = 2,
    Stopping = 3,
    Stopped = 4,
    Down = 5,
}

impl ServerStatus {
    ///
    pub fn to_string(&self) -> &'static str {
        match self {
            ServerStatus::Null => "kNull",
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
