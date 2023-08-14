use std::collections::LinkedList;

use super::ConnId;

///
pub static EMPTY_SLICE: &[u8; 0] = &[0_u8; 0];

/// Buffer size
const BUFFER_INITIAL_SIZE: usize = 4096;
const BUFFER_RESERVED_PREPEND_SIZE: usize = 8;

/// 协议号类型，2字节
pub type CmdId = u16;

/// 消息体加密字节数
const ENCRYPT_MAX_PKT_LEN: usize = 4; /* 4字节消息体 */
const PKT_CMD_LEN: usize = 2; /* 2字节协议号 */
const ENCRYPT_MAX_LEN: usize = PKT_CMD_LEN + ENCRYPT_MAX_PKT_LEN;
const ENCRYPT_KEY_LEN: usize = 64; /* 密钥总长度，根据 client no 进行偏移 */
const SAVED_NO_COUNT: usize = 1;

/// 4字节包头
const PKT_HEADER_LENGTH_SIZE: usize = 4;

/// 2字节包头(来自客户端)
const FROM_CLIENT_PKT_HEADER_LENGTH_SIZE: usize = 2;

/// 4字节包头 + 2字节协议号 // sizeof(data_size_) + sizeof(cmd_)
const SERVER_INNER_HEADER_SIZE: usize = PKT_HEADER_LENGTH_SIZE + 2;

/// 4字节包头 + 2字节协议号(发往客户端) // sizeof(data_size_) + sizeof(cmd_)
const TO_CLIENT_HEADER_SIZE: usize = PKT_HEADER_LENGTH_SIZE + 2;

/// 2字节包头 + 1字节序号 + 2字节协议号(来自客户端) // sizeof(data_size_) + sizeof(ClientHead::no_) + sizeof(cmd_)
const FROM_CLIENT_HEADER_SIZE: usize = FROM_CLIENT_PKT_HEADER_LENGTH_SIZE + 1 + 2;

/// 2字节协议号: WS
const TO_CLIENT_HEADER_SIZE_WS: usize = 2;

/// 1字节序号 + 2字节协议号: WS
const FROM_CLIENT_HEADER_SIZE_WS: usize = 3;

///
pub struct EncryptData {
    pub no_list: LinkedList<i8>, // 缓存的包序号列表
    pub encrypt_key: String,
}

///
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum PacketType {
    Server = 0, // 服务器内部包：不加密

    Client = 1, // 处理客户端包：收包解密，发包不加密
    Robot = 2,  // 模拟客户端包：发包加密，收包不需要解密

    ClientWs = 3, // 处理客户端包（WS）：收包解密，发包不加密
    RobotWs = 4,  // 模拟客户端包（WS）：发包加密，收包不需要解密
}

///
pub enum PacketSzie {
    Small = 0,
    Large = 1,
}

///
pub struct ClientHead {
    no: i8, // 包序号
}

/// 包头：
/// 服务器内部 => 4字节长度 + 2字节协议号
/// 客户端协议
///     客户端发包      => 2字节长度 + 1字节序号 + 2字节协议号
///     服务器到客户端包 => 4字节长度 + 2字节协议号
#[repr(C)]
pub struct NetPacket {
    packeg_size: PacketSzie,
    packet_type: PacketType,

    data_size: usize,
    cmd: CmdId,
    client: ClientHead,

    buffer: super::Buffer, // 完整包体
    buffer_raw_len: usize, // 完整包体原始长度
}

impl NetPacket {
    ///
    pub fn new() -> NetPacket {
        NetPacket {
            packeg_size: PacketSzie::Small,
            packet_type: PacketType::Server,

            data_size: 0,
            cmd: 0,
            client: ClientHead { no: 0 },

            buffer: super::Buffer::new(BUFFER_INITIAL_SIZE, BUFFER_RESERVED_PREPEND_SIZE),
            buffer_raw_len: 0,
        }
    }

    ///
    pub fn init(&mut self, _new_malloc: bool) {
        // 兼容内存池
    }

    ///
    pub fn release(&mut self) {
        self.data_size = 0;
        self.cmd = 0;
        self.client.no = 0;
        self.buffer.reset();
        self.buffer_raw_len = 0;
    }

    ///
    pub fn set_slice(&mut self, slice: &[u8]) {
        assert_eq!(self.buffer_raw_len, 0);
        let len = slice.len();
        self.buffer.write_slice(slice);
        self.buffer_raw_len = len;
        assert_eq!(self.buffer_raw_len, self.buffer.length());

        self.data_size = self.buffer_raw_len;
    }

    ///
    #[inline(always)]
    pub fn set_type(&mut self, packet_type: PacketType) {
        self.packet_type = packet_type
    }

    ///
    #[inline(always)]
    pub fn cmd(&self) -> CmdId {
        self.cmd
    }

    ///
    #[inline(always)]
    pub fn consume(&mut self) -> &[u8] {
        self.buffer.next_all()
    }

    ///
    pub fn set_msg<M>(&mut self, msg: &M)
    where
        M: prost::Message,
    {
        let len = msg.encoded_len();
        let pb_slice = self.buffer.next_of_write(len);
        write_prost_message(msg, pb_slice);

        self.buffer_raw_len = len;
        self.data_size = self.buffer_raw_len;
    }

    ///
    pub fn set_trans_msg<M>(&mut self, pid: crate::PlayerId, cmd: CmdId, msg: &M) -> bool
    where
        M: prost::Message,
    {
        self.buffer.append_u64(pid);
        self.buffer_raw_len += 4;

        self.buffer.append_u16(cmd);
        self.buffer_raw_len += 2;

        let len = msg.encoded_len();
        let pb_slice = self.buffer.next_of_write(len);
        write_prost_message(msg, pb_slice);
        self.buffer_raw_len += len;
        self.data_size = self.buffer_raw_len;
        true
    }

    ///
    pub fn set_multi_trans_msg<M>(
        &mut self,
        pids: Vec<crate::PlayerId>,
        cmd: CmdId,
        msg: &mut M,
    ) -> bool
    where
        M: prost::Message,
    {
        self.buffer.append_u16(pids.len() as u16);
        self.buffer_raw_len += 2;

        for pid in pids {
            self.buffer.append_u64(pid);
            self.buffer_raw_len += 4;
        }

        self.buffer.append_u16(cmd);
        self.buffer_raw_len += 2;

        let len = msg.encoded_len();
        let pb_slice = self.buffer.next_of_write(len);
        write_prost_message(msg, pb_slice);

        self.buffer_raw_len += len;
        self.data_size = self.buffer_raw_len;
        true
    }

    /// 包头长度校验
    pub fn check_packet(&self) -> bool {
        match self.packet_type {
            PacketType::Server => self.buffer.length() >= SERVER_INNER_HEADER_SIZE,
            PacketType::Client | PacketType::Robot => {
                self.buffer.length() >= FROM_CLIENT_HEADER_SIZE
            }
            PacketType::ClientWs | PacketType::RobotWs => {
                self.buffer.length() >= FROM_CLIENT_HEADER_SIZE_WS
            }
        }
    }

    /// Decode header from packet slice and return the data part of packet slice
    pub fn decode_packet(
        &mut self,
        hd: ConnId,
        encrypt_table: &mut hashbrown::HashMap<ConnId, EncryptData>,
    ) -> bool {
        let client_no = self.client.no;

        match self.packet_type {
            PacketType::Client => {
                // 解密
                if let Some(encrypt) = encrypt_table.get_mut(&hd) {
                    if !self.check_packet() {
                        // TODO: 是不是直接 close 这个连接？？？
                        log::error!(
                            "[decode_packet::PacketType::Client] received client data from [hd={:?}] error: check packet failed!!!",
                            hd
                        );

                        //
                        false
                    } else {
                        self.read_client_packet(encrypt.encrypt_key.as_str());

                        // TODO: 包序号检查
                        if !add_packet_no(encrypt, client_no) {
                            log::error!("[decode_packet::PacketType::Client] received client data from [hd={:?}] error: packet no {} already exist!!!",
                                hd, client_no
                            );

                            //
                            false
                        } else {
                            //
                            true
                        }
                    }
                } else {
                    log::error!(
                        "[decode_packet::PacketType::Client] received client data from [hd={:?}] error: encrypt data not exist!!!",
                        hd
                    );

                    //
                    false
                }
            }

            PacketType::ClientWs => {
                // 解密
                if let Some(encrypt) = encrypt_table.get_mut(&hd) {
                    if !self.check_packet() {
                        // TODO: 是不是直接 close 这个连接？？？
                        log::error!(
                            "[decode_packet::PacketType::ClientWs] received client data from [hd={:?}] error: check packet failed!!!",
                            hd
                        );

                        //
                        false
                    } else {
                        self.read_client_ws_packet(encrypt.encrypt_key.as_str());

                        // TODO: 包序号检查
                        if !add_packet_no(encrypt, client_no) {
                            log::error!("[decode_packet::PacketType::ClientWs] received client data from [hd={:?}] error: packet no {} already exist!!!",
                                hd, client_no);

                            //
                            false
                        } else {
                            //
                            true
                        }
                    }
                } else {
                    log::error!(
                        "[decode_packet::PacketType::ClientWs] received client data from [hd={:?}] error: encrypt data not exist!!!",
                        hd
                    );

                    //
                    false
                }
            }

            PacketType::Server => {
                self.read_server_packet();
                true
            }

            PacketType::Robot => {
                self.read_robot_packet();
                true
            }

            PacketType::RobotWs => {
                self.read_robot_ws_packet();
                true
            }
        }
    }

    /// Encode header into packet slice and return the full packet slice
    pub fn encode_packet(
        &mut self,
        hd: ConnId,
        encrypt_table: &mut hashbrown::HashMap<ConnId, EncryptData>,
    ) -> bool {
        match self.packet_type {
            PacketType::Robot => {
                // 加密
                if let Some(encrypt) = encrypt_table.get_mut(&hd) {
                    // 随机序号
                    let no = rand_packet_no(encrypt, hd);
                    self.client.no = no;
                    self.write_robot_packet(encrypt.encrypt_key.as_str());

                    //
                    true
                } else {
                    log::error!(
                        "[encode_packet::PacketType::Robot] [hd={:?}] send packet error: encrypt data not exist!!!",
                        hd
                    );

                    //
                    false
                }
            }

            PacketType::RobotWs => {
                // 加密
                if let Some(encrypt) = encrypt_table.get_mut(&hd) {
                    // 随机序号
                    let no = rand_packet_no(encrypt, hd);
                    self.client.no = no;
                    self.write_robot_ws_packet(encrypt.encrypt_key.as_str());

                    //
                    true
                } else {
                    log::error!(
                        "[encode_packet::PacketType::RobotWs] [hd={:?}] send packet error: encrypt data not exist!!!",
                        hd
                    );

                    //
                    false
                }
            }

            PacketType::Server => {
                self.write_server_packet();
                true
            }

            PacketType::Client => {
                self.write_client_packet();
                true
            }

            PacketType::ClientWs => {
                self.write_client_ws_packet();
                true
            }
        }
    }

    /* **** server **** */
    fn write_server_packet(&mut self) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);
        self.buffer_raw_len += 2;

        // 4 字节包长度
        let size = SERVER_INNER_HEADER_SIZE + self.data_size;
        self.buffer.prepend_u32(size as u32);
        self.buffer_raw_len += 4;
    }

    fn read_server_packet(&mut self) {
        // MUST BE only one packet in buffer
        assert_eq!(self.buffer_raw_len, self.buffer.length());

        // 4 字节长度
        self.data_size = self.buffer.read_u32() as usize;

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();

        self.data_size -= SERVER_INNER_HEADER_SIZE;
    }

    /* **** client **** */
    fn write_client_packet(&mut self) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);
        self.buffer_raw_len += 2;

        // 4 字节包长度
        let size = TO_CLIENT_HEADER_SIZE + self.data_size;
        self.buffer.prepend_u32(size as u32);
        self.buffer_raw_len += 4;
    }

    fn read_client_packet(&mut self, key: &str) {
        // MUST BE only one packet in buffer
        assert_eq!(self.buffer_raw_len, self.buffer.length());

        // 2 字节长度
        self.data_size = self.buffer.read_u16() as usize;
        self.data_size -= FROM_CLIENT_HEADER_SIZE;

        // 1 字节序号
        self.client.no = self.buffer.read_u8() as i8;

        // 解密
        decrypt_packet(self.buffer.data_mut(), self.data_size, key, self.client.no);

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();

        self.data_size -= SERVER_INNER_HEADER_SIZE;
    }

    /* **** robot **** */
    fn write_robot_packet(&mut self, key: &str) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);
        self.buffer_raw_len += 2;

        // 加密
        encrypt_packet(self.buffer.data_mut(), self.data_size, key, self.client.no);

        // 1 字节序号
        self.buffer.prepend_u8(self.client.no as u8);
        self.buffer_raw_len += 1;

        // 2 字节包长度
        let size = FROM_CLIENT_HEADER_SIZE + self.data_size;
        self.buffer.prepend_u16(size as u16);
        self.buffer_raw_len += 2;
    }

    fn read_robot_packet(&mut self) {
        // MUST BE only one packet in buffer
        assert_eq!(self.buffer_raw_len, self.buffer.length());

        // 4 字节长度
        self.data_size = self.buffer.read_u32() as usize;

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();

        self.data_size -= TO_CLIENT_HEADER_SIZE;
    }

    /* **** client ws **** */
    fn write_client_ws_packet(&mut self) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);
        self.buffer_raw_len += 2;
    }

    fn read_client_ws_packet(&mut self, key: &str) {
        // MUST BE only one packet in buffer
        assert_eq!(self.buffer_raw_len, self.buffer.length());

        //
        self.data_size = self.buffer_raw_len - FROM_CLIENT_HEADER_SIZE_WS;

        // 1 字节序号
        self.client.no = self.buffer.read_u8() as i8;

        // 解密
        decrypt_packet(self.buffer.data_mut(), self.data_size, key, self.client.no);

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();
    }

    /* **** robot ws **** */
    fn write_robot_ws_packet(&mut self, key: &str) {
        // 组合最终包 (Notice: Prepend 是反向添加)
        // 2 字节 cmd
        self.buffer.prepend_u16(self.cmd);
        self.buffer_raw_len += 2;

        // 加密
        encrypt_packet(self.buffer.data_mut(), self.data_size, key, self.client.no);

        // 1 字节序号
        self.buffer.prepend_u8(self.client.no as u8);
        self.buffer_raw_len += 1;
    }

    fn read_robot_ws_packet(&mut self) {
        // MUST BE only one packet in buffer
        assert_eq!(self.buffer_raw_len, self.buffer.length());

        //
        self.data_size = self.buffer_raw_len - TO_CLIENT_HEADER_SIZE_WS;

        // 2 字节 cmd
        self.cmd = self.buffer.read_u16();
    }
}

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

#[inline(always)]
pub fn rand_packet_no(encrypt: &mut EncryptData, _hd: ConnId) -> i8 {
    let no_list = &mut encrypt.no_list;
    let no = crate::rand_between_exclusive_i8(0, (ENCRYPT_KEY_LEN - 1) as i8, no_list);

    no_list.push_back(no);

    if no_list.len() > SAVED_NO_COUNT {
        no_list.pop_front();
    }
    no
}

#[inline(always)]
pub fn add_packet_no(encrypt: &mut EncryptData, no: i8) -> bool {
    if no >= ENCRYPT_KEY_LEN as i8 {
        return false;
    }

    let no_list = &mut encrypt.no_list;
    for it in &*no_list {
        if *it == no {
            return false;
        }
    }

    no_list.push_back(no);

    if no_list.len() > SAVED_NO_COUNT {
        no_list.pop_front();
    }
    true
}

#[inline(always)]
fn encrypt_packet(data: *mut u8, len: usize, key: &str, no: i8) {
    let slice_len = if len < ENCRYPT_MAX_PKT_LEN {
        PKT_CMD_LEN + len
    } else {
        ENCRYPT_MAX_LEN
    };

    let key_len = ENCRYPT_KEY_LEN - no as usize;

    let encrypt_len = std::cmp::min(slice_len, key_len);
    let slice = unsafe { std::slice::from_raw_parts_mut(data, encrypt_len) };
    for i in 0..encrypt_len {
        let from = no as usize + i;
        slice[i] ^= (key.as_bytes())[from];
    }
}

#[inline(always)]
fn decrypt_packet(data: *mut u8, len: usize, key: &str, no: i8) {
    // xor decrypt is just same as encrypt
    encrypt_packet(data, len, key, no);
}
