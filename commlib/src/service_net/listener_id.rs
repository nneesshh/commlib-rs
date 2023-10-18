/// Tcp server id
#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct ListenerId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl ListenerId {}

impl From<usize> for ListenerId {
    #[inline(always)]
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}

impl std::fmt::Display for ListenerId {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
