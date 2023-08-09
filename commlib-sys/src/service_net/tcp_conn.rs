use std::rc::Rc;

#[repr(C)]
pub struct TcpConn {
    packet_type: u32,
}

impl TcpConn {
    ///
    pub fn new() -> TcpConn {
        TcpConn { packet_type: 0 }
    }
}
