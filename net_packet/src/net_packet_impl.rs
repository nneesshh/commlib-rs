use super::Buffer;

/// Buffer size
//pub const BUFFER_INITIAL_SIZE: usize = 4096;
pub const BUFFER_INITIAL_SIZE: usize = 96; // debug only
pub const BUFFER_RESERVED_PREPEND_SIZE: usize = 8;

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
    pub body_size: usize, // 包体纯数据长度，不包含包头（包头：包体前导长度字段，协议号，包序号等）
    pub cmd: CmdId,
    pub client: ClientHead,

    pub buffer: Buffer, // 包体数据缓冲区
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

    /// 扩展 write 缓冲空间
    #[inline(always)]
    pub fn extend(&mut self, len: usize) -> &mut [u8] {
        self.buffer.extend(len)
    }

    /// 截断 write 缓冲空间
    #[inline(always)]
    pub fn truncate(&mut self, remain_n: usize) {
        self.buffer.truncate_to(remain_n);
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
        self.buffer.discard(n)
    }

    ///
    #[inline(always)]
    pub fn set_body(&mut self, slice: &[u8]) {
        let len = slice.len();
        self.buffer.write_slice(slice);
        self.body_size = len;
    }

    /// 包头长度校验
    #[inline(always)]
    pub fn check_packet(&self) -> bool {
        self.buffer.length() >= self.leading_field_size() as usize
    }
}
