use bytemuck::NoUninit;
use std::collections::LinkedList;

use crate::PlayerId;

use super::Buffer;
use super::{decrypt_packet, encrypt_packet};

/// Buffer size
pub const BUFFER_INITIAL_SIZE: usize = 4096;
pub const BUFFER_RESERVED_PREPEND_SIZE: usize = 8;

/// 协议号类型，2字节
pub type CmdId = u16;
pub const PKT_CMD_LEN: usize = 2; /* 2字节协议号 */

/// 4字节包体前导长度字段
const PKT_LEADING_FIELD_SIZE_DEFAULT: usize = 4;

/// 2字节包体前导长度字段(来自客户端)
const FROM_CLIENT_PKT_LEADING_FIELD_SIZE: usize = 2;

/// 4字节包体前导长度字段 + 2字节协议号
///     leading(pkt_full_len)(4) + cmd(2)
const SERVER_INNER_HEADER_SIZE: usize = PKT_LEADING_FIELD_SIZE_DEFAULT + 2;

/// 4字节包体前导长度字段 + 2字节协议号(发往客户端)
///     leading( pkt_full_len)(4) + cmd(2)
const TO_CLIENT_HEADER_SIZE: usize = PKT_LEADING_FIELD_SIZE_DEFAULT + 2;

/// 2字节包体前导长度字段 + 1字节序号 + 2字节协议号(来自客户端)
///     leading(pkt_full_len)(2) + client_no(1) + cmd(2)
const FROM_CLIENT_HEADER_SIZE: usize = FROM_CLIENT_PKT_LEADING_FIELD_SIZE + 1 + 2;

/// 2字节协议号: WS
///     cmd(2)
const TO_CLIENT_HEADER_SIZE_WS: usize = 2;

/// 1字节序号 + 2字节协议号: WS
///     client_no(1) + cmd(2)
const FROM_CLIENT_HEADER_SIZE_WS: usize = 3;

///
pub struct EncryptData {
    pub no_list: LinkedList<i8>, // 缓存的包序号列表
    pub encrypt_key: String,
}

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
pub enum PacketSizeType {
    Small = 0,
    Large = 1,
}

/// 客户端专用
pub struct ClientHead {
    no: i8, // 包序号, MUTS be less than 128
}

/// 包头前导长度字段：
/// 服务器内部 => 4字节长度 + 2字节协议号
/// 客户端协议
///     客户端发包      => 2字节长度 + 1字节序号 + 2字节协议号
///     服务器到客户端包 => 4字节长度 + 2字节协议号
#[repr(C)]
pub struct NetPacket {
    size_type: PacketSizeType,
    leading_field_size: u8,

    ///
    body_size: usize, // 包体纯数据长度，不包含包头（包头：包体前导长度字段，协议号，包序号等）
    cmd: CmdId,
    client: ClientHead,

    buffer: Buffer, // 包体数据缓冲区
}

impl NetPacket {
    ///
    pub fn new() -> Self {
        Self {
            size_type: PacketSizeType::Small,
            leading_field_size: 4, // 缺省占用4字节

            body_size: 0,
            cmd: 0,
            client: ClientHead { no: 0 },

            buffer: Buffer::new(BUFFER_INITIAL_SIZE, BUFFER_RESERVED_PREPEND_SIZE),
        }
    }

    ///
    #[inline(always)]
    pub fn init(&mut self, _new_malloc: bool) {
        // 兼容内存池
    }

    ///
    #[inline(always)]
    pub fn release(&mut self) {
        self.body_size = 0;
        self.cmd = 0;
        self.set_client_no(0);
        self.buffer.reset();
    }

    /// 向 pkt 追加数据
    #[inline(always)]
    pub fn append(&mut self, data: *const u8, len: usize) {
        //
        self.buffer.write(data, len);

        // body size 计数
        let leading_size = self.leading_field_size as usize;
        self.body_size = if self.buffer_raw_len() >= leading_size {
            self.buffer_raw_len() - leading_size
        } else {
            0
        };
    }

    /// 向 pkt 追加数据
    #[inline(always)]
    pub fn append_slice(&mut self, slice: &[u8]) {
        //
        self.buffer.write_slice(slice);

        // body size 计数
        let leading_size = self.leading_field_size() as usize;
        self.body_size = if self.buffer_raw_len() >= leading_size {
            self.buffer_raw_len() - leading_size
        } else {
            0
        };
    }

    #[inline(always)]
    pub fn set_size_type(&mut self, size_type: PacketSizeType) {
        self.size_type = size_type;
    }

    /// 包体前导长度字段位数
    #[inline(always)]
    pub fn leading_field_size(&self) -> u8 {
        self.leading_field_size
    }

    ///
    #[inline(always)]
    pub fn set_leading_field_size(&mut self, leading_field_size: u8) {
        self.leading_field_size = leading_field_size;
    }

    /// 包体数据缓冲区 剩余可写容量
    #[inline(always)]
    pub fn buffer_writable_bytes(&self) -> usize {
        self.buffer.writable_bytes()
    }

    /// 确保包体数据缓冲区 剩余可写容量
    #[inline(always)]
    pub fn ensure_writable_bytes(&mut self, len: usize) {
        self.buffer.ensure_writable_bytes(len);
    }

    /// 包体数据缓冲区尚未读取的数据数量
    #[inline(always)]
    pub fn buffer_raw_len(&self) -> usize {
        self.buffer.length()
    }

    /// 协议号
    #[inline(always)]
    pub fn cmd(&self) -> CmdId {
        self.cmd
    }

    ///
    #[inline(always)]
    pub fn set_cmd(&mut self, cmd: CmdId) {
        self.cmd = cmd;
    }

    /// 包序号, MUTS be less than 128
    #[inline(always)]
    pub fn client_no(&self) -> i8 {
        self.client.no
    }

    ///
    #[inline(always)]
    pub fn set_client_no(&mut self, client_no: i8) {
        self.client.no = client_no;
    }

    /// 查看 buffer 数据
    pub fn peek(&self) -> &[u8] {
        self.buffer.peek()
    }

    /// 查看 buffer n 个字节
    pub fn peek_leading_field(&self) -> usize {
        // 客户端包 2 字节包头，其他都是 4 字节包头
        if 2 == self.leading_field_size {
            self.buffer.peek_u16() as usize
        } else {
            self.buffer.peek_u32() as usize
        }
    }

    /// 内部消耗掉 buffer 数据，供给外部使用
    #[inline(always)]
    pub fn consume(&mut self) -> &[u8] {
        self.buffer.next_all()
    }

    /// 内部消耗掉 buffer 数据，供给外部使用
    #[inline(always)]
    pub fn consume_n(&mut self, n: usize) -> &[u8] {
        self.buffer.next(n)
    }

    /// 内部消耗掉 buffer 数据，供给外部使用
    #[inline(always)]
    pub fn consume_tail_n(&mut self, n: usize) -> &[u8] {
        self.buffer.shrink(n)
    }

    ///
    #[inline(always)]
    pub fn set_body(&mut self, slice: &[u8]) {
        let len = slice.len();
        self.buffer.write_slice(slice);
        self.body_size = len;
    }

    ///
    #[inline(always)]
    pub fn set_msg<M>(&mut self, msg: &M)
    where
        M: prost::Message,
    {
        let len = msg.encoded_len();
        let pb_slice = self.buffer.next_of_write(len);
        write_prost_message(msg, pb_slice);

        self.body_size = len;
    }

    ///
    #[inline(always)]
    pub fn set_trans_msg<M>(&mut self, pid: PlayerId, cmd: CmdId, msg: &M) -> bool
    where
        M: prost::Message,
    {
        // 4字节 pid
        self.buffer.append_u64(pid);

        // 2字节 cmd
        self.buffer.append_u16(cmd);

        let len = msg.encoded_len();
        let pb_slice = self.buffer.next_of_write(len);
        write_prost_message(msg, pb_slice);

        self.body_size = len;
        true
    }

    ///
    #[inline(always)]
    pub fn set_multi_trans_msg<M>(&mut self, pids: Vec<PlayerId>, cmd: CmdId, msg: &mut M) -> bool
    where
        M: prost::Message,
    {
        // 2字节 pids 列表长度
        self.buffer.append_u16(pids.len() as u16);

        for pid in pids {
            // 4字节 pid
            self.buffer.append_u64(pid);
        }

        // 2字节 cmd
        self.buffer.append_u16(cmd);

        let len = msg.encoded_len();
        let pb_slice = self.buffer.next_of_write(len);
        write_prost_message(msg, pb_slice);

        self.body_size = len;
        true
    }

    /// 包头长度校验
    #[inline(always)]
    pub fn check_packet(&self) -> bool {
        self.buffer.length() >= self.leading_field_size() as usize
    }

    ///
    #[inline(always)]
    pub fn write_server_packet(&mut self) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);

        // 4 字节包长度
        let size = SERVER_INNER_HEADER_SIZE + self.body_size;
        self.buffer.prepend_u32(size as u32);
    }

    ///
    #[inline(always)]
    pub fn read_server_packet(&mut self) {
        // MUST only one packet in buffer

        // 4 字节长度
        let pkt_full_len = self.buffer.read_u32() as usize;
        self.body_size = pkt_full_len - SERVER_INNER_HEADER_SIZE;

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();
    }

    ///
    #[inline(always)]
    pub fn write_client_packet(&mut self) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);

        // 4 字节包长度
        let size = TO_CLIENT_HEADER_SIZE + self.body_size;
        self.buffer.prepend_u32(size as u32);
    }

    ///
    #[inline(always)]
    pub fn read_client_packet(&mut self, key: &str) {
        // MUST only one packet in buffer

        // 2 字节长度
        let pkt_full_len = self.buffer.read_u16() as usize;
        self.body_size = pkt_full_len - FROM_CLIENT_HEADER_SIZE;

        // 1 字节序号
        let no = self.buffer.read_u8() as i8;
        self.set_client_no(no);

        // 解密
        decrypt_packet(
            self.buffer.data_mut(),
            self.body_size,
            key,
            self.client_no(),
        );

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();
    }

    ///
    #[inline(always)]
    pub fn write_robot_packet(&mut self, key: &str) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);

        // 加密
        encrypt_packet(
            self.buffer.data_mut(),
            self.body_size,
            key,
            self.client_no(),
        );

        // 1 字节序号
        self.buffer.prepend_u8(self.client_no() as u8);

        // 2 字节包长度
        let size = FROM_CLIENT_HEADER_SIZE + self.body_size;
        self.buffer.prepend_u16(size as u16);
    }

    ///
    #[inline(always)]
    pub fn read_robot_packet(&mut self) {
        // MUST only one packet in buffer

        // 4 字节长度
        let pkt_full_len = self.buffer.read_u32() as usize;
        self.body_size = pkt_full_len - TO_CLIENT_HEADER_SIZE;

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();
    }

    ///
    #[inline(always)]
    pub fn write_client_ws_packet(&mut self) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);
    }

    ///
    #[inline(always)]
    pub fn read_client_ws_packet(&mut self, key: &str) {
        // MUST only one packet in buffer

        //
        self.body_size = self.buffer_raw_len() - FROM_CLIENT_HEADER_SIZE_WS;

        // 1 字节序号
        let no = self.buffer.read_u8() as i8;
        self.set_client_no(no);

        // 解密
        decrypt_packet(
            self.buffer.data_mut(),
            self.body_size,
            key,
            self.client_no(),
        );

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();
    }

    ///
    #[inline(always)]
    pub fn write_robot_ws_packet(&mut self, key: &str) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);

        // 加密
        encrypt_packet(
            self.buffer.data_mut(),
            self.body_size,
            key,
            self.client_no(),
        );

        // 1 字节序号
        self.buffer.prepend_u8(self.client_no() as u8);
    }

    ///
    #[inline(always)]
    pub fn read_robot_ws_packet(&mut self) {
        // MUST only one packet in buffer

        //
        self.body_size = self.buffer_raw_len() - TO_CLIENT_HEADER_SIZE_WS;

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();
    }
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

///
#[inline(always)]
pub fn write_prost_message<M>(msg: &M, mut buf: &mut [u8]) -> bool
where
    M: prost::Message,
{
    match msg.encode(&mut buf) {
        Ok(()) => true,
        Err(err) => {
            log::error!("encode msg error: {}!!! {:?},", err, msg);
            false
        }
    }
}
