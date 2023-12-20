use std::io::{self, Cursor};

use super::Buffer;

/// Buffer size
pub const BUFFER_INITIAL_SIZE: usize = 4096;
pub const BUFFER_HEADER_RESERVE_SIZE: usize = 8;

/// 协议号类型，2字节
pub type CmdId = u16;

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
    pub size_type: PacketSizeType,
    pub leading_field_size: u8,

    ///
    pub cmd: CmdId,
    pub client: ClientHead,

    buffer: Buffer, // 包体数据缓冲区
}

impl NetPacket {
    ///
    pub fn new() -> Self {
        Self {
            size_type: PacketSizeType::Small,
            leading_field_size: 4, // 缺省占用4字节

            cmd: 0,
            client: ClientHead { no: 0 },

            buffer: Buffer::new(BUFFER_INITIAL_SIZE, BUFFER_HEADER_RESERVE_SIZE),
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
        self.cmd = 0;
        self.set_client_no(0);
        self.buffer.reset();
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

    /// 包体数据缓冲区的 body 长度（包体缓冲区 = header + body ), for write only
    #[inline(always)]
    pub fn wrote_body_len(&self) -> usize {
        self.buffer.wrote_body_len()
    }

    /// 包体数据缓冲区尚未读取的数据数量
    #[inline(always)]
    pub fn buffer_raw_len(&self) -> usize {
        self.buffer.size()
    }

    /// 包体数据缓冲区尚未读取的数据数量 是否 为空
    #[inline(always)]
    pub fn has_remaining(&self) -> bool {
        !self.is_empty()
    }

    ///
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
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

    /// 包头长度校验
    #[inline(always)]
    pub fn check_packet(&self) -> bool {
        self.buffer.size() >= self.leading_field_size() as usize
    }
}

/* **************** */
// for write packet
/* **************** */
impl NetPacket {
    /// 包体数据缓冲区 剩余可写容量
    #[inline(always)]
    pub fn free_space(&self) -> usize {
        self.buffer.free_space()
    }

    /// 确保包体数据缓冲区 剩余可写容量
    #[inline(always)]
    pub fn ensure_free_space(&mut self, len: usize) {
        self.buffer.ensure_free_space(len);
    }

    /// write 缓冲空间
    #[inline(always)]
    pub fn as_write_mut(&mut self) -> &mut [u8] {
        self.buffer.as_write_mut()
    }

    /// write end，重设缓冲空间长度
    #[inline(always)]
    pub fn end_write(&mut self, len: usize) -> u64 {
        let w_pos = self.buffer.write_pos();
        let new_w_pos = w_pos + len as u64;
        self.buffer.set_write_pos(new_w_pos);
        new_w_pos
    }

    /// 向 pkt 追加数据
    #[inline(always)]
    pub fn append(&mut self, data: *const u8, len: usize) {
        let slice = unsafe { std::slice::from_raw_parts(data, len) };
        self.append_slice(slice);
    }

    /// 向 pkt 追加数据 (slice)
    #[inline(always)]
    pub fn append_slice(&mut self, slice: &[u8]) {
        //
        self.buffer.write_slice(slice);
    }

    /// Append: u128
    #[inline(always)]
    pub fn append_u128(&mut self, n: u128) {
        self.buffer.append_u128(n);
    }

    /// Append: u64
    #[inline(always)]
    pub fn append_u64(&mut self, n: u64) {
        self.buffer.append_u64(n);
    }

    /// Append: u32
    #[inline(always)]
    pub fn append_u32(&mut self, n: u32) {
        self.buffer.append_u32(n);
    }

    /// Append: u16
    #[inline(always)]
    pub fn append_u16(&mut self, n: u16) {
        self.buffer.append_u16(n);
    }

    /// Append: u8
    #[inline(always)]
    pub fn append_u8(&mut self, n: u8) {
        self.buffer.append_u8(n);
    }
}

/* **************** */
// for read packet
/* **************** */
impl NetPacket {
    ///
    #[inline(always)]
    pub fn advance(&mut self, cnt: usize) -> &mut [u8] {
        self.buffer.advance(cnt)
    }

    /// 内部消耗掉 buffer 数据，供给外部使用
    #[inline(always)]
    pub fn consume(&mut self) -> &mut [u8] {
        self.buffer.advance_all()
    }

    /// 内部消耗掉 buffer 数据，供给外部使用
    #[inline(always)]
    pub fn consume_n(&mut self, n: usize) -> &mut [u8] {
        self.advance(n)
    }

    /// 内部消耗掉 buffer 数据，供给外部使用
    #[inline(always)]
    pub fn consume_tail_n(&mut self, n: usize) -> &[u8] {
        self.buffer.discard(n)
    }

    ///
    #[inline(always)]
    pub fn as_read_mut(&mut self) -> &mut [u8] {
        self.buffer.as_read_mut()
    }

    /// Cursor mut for read
    #[inline(always)]
    pub fn as_cursor_mut(&mut self) -> &mut Cursor<Vec<u8>> {
        self.buffer.as_cursor_mut()
    }

    /// Prepend: u128
    #[inline(always)]
    pub fn prepend_u128(&mut self, n: u128) {
        self.buffer.prepend_u128(n);
    }

    /// Prepend: u64
    #[inline(always)]
    pub fn prepend_u64(&mut self, n: u64) {
        self.buffer.prepend_u64(n);
    }

    /// Prepend: u32
    #[inline(always)]
    pub fn prepend_u32(&mut self, n: u32) {
        self.buffer.prepend_u32(n);
    }

    /// Prepend: u16
    #[inline(always)]
    pub fn prepend_u16(&mut self, n: u16) {
        self.buffer.prepend_u16(n);
    }

    /// Prepend: u8
    #[inline(always)]
    pub fn prepend_u8(&mut self, n: u8) {
        self.buffer.prepend_u8(n);
    }

    /// Read and consume: u128
    #[inline(always)]
    pub fn read_u128(&mut self) -> u128 {
        self.buffer.read_u128()
    }

    /// Read and consume: u64
    #[inline(always)]
    pub fn read_u64(&mut self) -> u64 {
        self.buffer.read_u64()
    }

    /// Read and consume: u32
    #[inline(always)]
    pub fn read_u32(&mut self) -> u32 {
        self.buffer.read_u32()
    }

    /// Read and consume: u16
    #[inline(always)]
    pub fn read_u16(&mut self) -> u16 {
        self.buffer.read_u16()
    }

    /// Read and consume: u8
    #[inline(always)]
    pub fn read_u8(&mut self) -> u8 {
        self.buffer.read_u8()
    }
}

/* **************** */
// for stream
/* **************** */
impl NetPacket {
    ///
    #[inline(always)]
    pub fn read_from<S: io::Read>(&mut self, stream: &mut S) -> io::Result<usize> {
        self.buffer.read_from(stream)
    }
}

impl std::fmt::Write for NetPacket {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), std::fmt::Error> {
        self.buffer.write_slice(s.as_bytes());
        Ok(())
    }
}
