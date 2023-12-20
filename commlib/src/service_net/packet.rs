use bytemuck::NoUninit;

/// 4字节包体前导长度字段
#[allow(dead_code)]
pub const PKT_LEADING_FIELD_SIZE_DEFAULT: usize = 4;

/// 2字节包体前导长度字段(来自客户端)
#[allow(dead_code)]
pub const FROM_CLIENT_PKT_LEADING_FIELD_SIZE: usize = 2;

/// 包类型
#[derive(Debug, PartialEq, Copy, Clone, NoUninit)]
#[repr(u8)]
pub enum PacketType {
    Server = 0, // 服务器内部包：不加密

    Client = 1, // 处理客户端包：收包解密，发包不加密
    Robot = 2,  // 模拟客户端包：发包加密，收包不需要解密

    ClientWs = 3, // 处理客户端包（WS）：收包解密，发包不加密
    RobotWs = 4,  // 模拟客户端包（WS）：发包加密，收包不需要解密

    Redis = 5, // Redis客户端
}

///
#[inline(always)]
pub fn get_leading_field_size(packet_type: PacketType) -> u8 {
    // 客户端包 2 字节包头，其他都是 4 字节包头
    match packet_type {
        PacketType::Client => FROM_CLIENT_PKT_LEADING_FIELD_SIZE as u8,
        _ => PKT_LEADING_FIELD_SIZE_DEFAULT as u8,
    }
}
