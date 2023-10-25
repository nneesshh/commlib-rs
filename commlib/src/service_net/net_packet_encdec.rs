use bytemuck::NoUninit;
use std::collections::LinkedList;

use message_io::net_packet::NetPacketGuard;

use crate::rand_between_exclusive_i8;

use super::ConnId;

/// 协议号类型，2字节
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
pub struct EncryptData {
    pub no_list: LinkedList<i8>, // 缓存的包序号列表
    pub encrypt_key: String,
}

/// 消息体加密字节数
const ENCRYPT_MAX_BODY_LEN: usize = 4; /* 4字节消息体 */
pub const ENCRYPT_MAX_LEN: usize = PKT_CMD_LEN + ENCRYPT_MAX_BODY_LEN;
pub const ENCRYPT_KEY_LEN: usize = 64; /* 密钥总长度，根据 client no 进行偏移 */
const SAVED_NO_COUNT: usize = 1;

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

///
#[inline(always)]
pub fn encrypt_packet(data: *mut u8, len: usize, key: &str, no: i8) {
    let slice_len = if len < ENCRYPT_MAX_BODY_LEN {
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

///
#[inline(always)]
pub fn decrypt_packet(data: *mut u8, len: usize, key: &str, no: i8) {
    // xor decrypt is just same as encrypt
    encrypt_packet(data, len, key, no);
}

/// Decode header from packet slice
pub fn decode_packet(
    packet_type: PacketType,
    hd: ConnId,
    pkt: &mut NetPacketGuard,
    encrypt_table: &mut hashbrown::HashMap<ConnId, EncryptData>,
) -> bool {
    match packet_type {
        PacketType::Client => {
            // 解密
            let encrypt_opt = encrypt_table.get_mut(&hd);
            if let Some(encrypt) = encrypt_opt {
                if !pkt.check_packet() {
                    // TODO: 是不是直接 close 这个连接？？？
                    log::error!(
                        "[decode_packet][PacketType::Client][hd={}] error: check packet failed!!!",
                        hd
                    );

                    //
                    false
                } else {
                    //
                    read_client_packet(pkt, encrypt.encrypt_key.as_str());

                    // TODO: 包序号检查
                    let client_no = pkt.client_no();
                    if !add_packet_no(encrypt, client_no) {
                        log::error!("[decode_packet][PacketType::Client][hd={}] error: packet no {} already exist!!!",
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
                    "[decode_packet][PacketType::Client][hd={}] error: encrypt data not exist!!!",
                    hd
                );

                //
                false
            }
        }

        PacketType::ClientWs => {
            // 解密
            let encrypt_opt = encrypt_table.get_mut(&hd);
            if let Some(encrypt) = encrypt_opt {
                if !pkt.check_packet() {
                    // TODO: 是不是直接 close 这个连接？？？
                    log::error!(
                            "[decode_packet][PacketType::ClientWs][hd={}] error: check packet failed!!!",
                            hd
                        );

                    //
                    false
                } else {
                    //
                    read_client_ws_packet(pkt, encrypt.encrypt_key.as_str());

                    // TODO: 包序号检查
                    let client_no = pkt.client_no();
                    if !add_packet_no(encrypt, client_no) {
                        log::error!("[decode_packet][PacketType::ClientWs][hd={}] error: packet no {} already exist!!!",
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
                    "[decode_packet][PacketType::ClientWs][hd={}] error: encrypt data not exist!!!",
                    hd
                );

                //
                false
            }
        }

        PacketType::Server => {
            read_server_packet(pkt);
            true
        }

        PacketType::Robot => {
            read_robot_packet(pkt);
            true
        }

        PacketType::RobotWs => {
            read_robot_ws_packet(pkt);
            true
        }

        _ => {
            std::unreachable!()
        }
    }
}

/// Encode header into packet slice
pub fn encode_packet(
    packet_type: PacketType,
    hd: ConnId,
    pkt: &mut NetPacketGuard,
    encrypt_table: &mut hashbrown::HashMap<ConnId, EncryptData>,
) -> bool {
    match packet_type {
        PacketType::Robot => {
            // 加密
            let encrypt_opt = encrypt_table.get_mut(&hd);
            if let Some(encrypt) = encrypt_opt {
                // 随机序号
                let no = rand_packet_no(encrypt, hd);
                pkt.set_client_no(no);
                write_robot_packet(pkt, encrypt.encrypt_key.as_str());

                log::info!(
                    "[encode_packet][PacketType::Robot][hd={}] rand_packet_no: {}",
                    hd,
                    no,
                );

                //
                true
            } else {
                log::error!(
                    "[encode_packet][PacketType::Robot][hd={}] encrypt data not exist!!!",
                    hd
                );

                //
                false
            }
        }

        PacketType::RobotWs => {
            // 加密
            let encrypt_opt = encrypt_table.get_mut(&hd);
            if let Some(encrypt) = encrypt_opt {
                // 随机序号
                let no = rand_packet_no(encrypt, hd);
                pkt.set_client_no(no);
                write_robot_ws_packet(pkt, encrypt.encrypt_key.as_str());

                log::info!(
                    "[encode_packet][PacketType::RobotWs][hd={}] rand_packet_no: {}",
                    hd,
                    no,
                );

                //
                true
            } else {
                log::error!(
                    "[encode_packet][PacketType::RobotWs][hd={}] encrypt data not exist!!!",
                    hd
                );

                //
                false
            }
        }

        PacketType::Server => {
            write_server_packet(pkt);
            true
        }

        PacketType::Client => {
            write_client_packet(pkt);
            true
        }

        PacketType::ClientWs => {
            write_client_ws_packet(pkt);
            true
        }

        _ => {
            //
            std::unreachable!()
        }
    }
}

#[inline(always)]
fn rand_packet_no(encrypt: &mut EncryptData, _hd: ConnId) -> i8 {
    let no_list = &mut encrypt.no_list;
    let no = rand_between_exclusive_i8(0, (ENCRYPT_KEY_LEN - 1) as i8, no_list);

    no_list.push_back(no);

    if no_list.len() > SAVED_NO_COUNT {
        no_list.pop_front();
    }
    no
}

#[inline(always)]
fn add_packet_no(encrypt: &mut EncryptData, no: i8) -> bool {
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

/* //////////////////////////////////////////////////////////////// */

#[inline(always)]
fn write_server_packet(pkt: &mut NetPacketGuard) {
    // 组合最终包 (Notice: Prepend 是反向添加)
    // 2 字节 cmd
    let cmd = pkt.cmd as u16;
    pkt.buffer.prepend_u16(cmd);

    // 4 字节包长度
    let size = SERVER_INNER_HEADER_SIZE + pkt.body_size;
    pkt.buffer.prepend_u32(size as u32);
}

///
#[inline(always)]
fn read_server_packet(pkt: &mut NetPacketGuard) {
    // MUST only one packet in buffer

    // 4 字节长度
    let pkt_full_len = pkt.buffer.read_u32() as usize;
    pkt.body_size = pkt_full_len - SERVER_INNER_HEADER_SIZE;

    // 2 字节 cmd
    pkt.cmd = pkt.buffer.read_u16();
}

///
#[inline(always)]
fn write_client_packet(pkt: &mut NetPacketGuard) {
    // 组合最终包 (Notice: Prepend 是反向添加)
    // 2 字节 cmd
    let cmd = pkt.cmd as u16;
    pkt.buffer.prepend_u16(cmd);

    // 4 字节包长度
    let size = TO_CLIENT_HEADER_SIZE + pkt.body_size;
    pkt.buffer.prepend_u32(size as u32);
}

///
#[inline(always)]
fn read_client_packet(pkt: &mut NetPacketGuard, key: &str) {
    // MUST only one packet in buffer

    // 2 字节长度
    let pkt_full_len = pkt.buffer.read_u16() as usize;
    pkt.body_size = pkt_full_len - FROM_CLIENT_HEADER_SIZE;

    // 1 字节序号
    let no = pkt.buffer.read_u8() as i8;
    pkt.set_client_no(no);

    // 解密
    decrypt_packet(pkt.buffer.data_mut(), pkt.body_size, key, pkt.client_no());

    // 2 字节 cmd
    pkt.cmd = pkt.buffer.read_u16();
}

///
#[inline(always)]
fn write_robot_packet(pkt: &mut NetPacketGuard, key: &str) {
    // 组合最终包 (Notice: Prepend 是反向添加)
    // 2 字节 cmd
    let cmd = pkt.cmd as u16;
    pkt.buffer.prepend_u16(cmd);

    // 加密
    encrypt_packet(pkt.buffer.data_mut(), pkt.body_size, key, pkt.client_no());

    // 1 字节序号
    let no = pkt.client_no() as u8;
    pkt.buffer.prepend_u8(no);

    // 2 字节包长度
    let size = FROM_CLIENT_HEADER_SIZE + pkt.body_size;
    pkt.buffer.prepend_u16(size as u16);
}

///
#[inline(always)]
fn read_robot_packet(pkt: &mut NetPacketGuard) {
    // MUST only one packet in buffer

    // 4 字节长度
    let pkt_full_len = pkt.buffer.read_u32() as usize;
    pkt.body_size = pkt_full_len - TO_CLIENT_HEADER_SIZE;

    // 2 字节 cmd
    pkt.cmd = pkt.buffer.read_u16();
}

///
#[inline(always)]
fn write_client_ws_packet(pkt: &mut NetPacketGuard) {
    // 组合最终包 (Notice: Prepend 是反向添加)
    // 2 字节 cmd
    let cmd = pkt.cmd as u16;
    pkt.buffer.prepend_u16(cmd);
}

///
#[inline(always)]
fn read_client_ws_packet(pkt: &mut NetPacketGuard, key: &str) {
    // MUST only one packet in buffer

    //
    pkt.body_size = pkt.buffer_raw_len() - FROM_CLIENT_HEADER_SIZE_WS;

    // 1 字节序号
    let no = pkt.buffer.read_u8() as i8;
    pkt.set_client_no(no);

    // 解密
    decrypt_packet(pkt.buffer.data_mut(), pkt.body_size, key, pkt.client_no());

    // 2 字节 cmd
    pkt.cmd = pkt.buffer.read_u16();
}

///
#[inline(always)]
fn write_robot_ws_packet(pkt: &mut NetPacketGuard, key: &str) {
    // 组合最终包 (Notice: Prepend 是反向添加)
    // 2 字节 cmd
    let cmd = pkt.cmd as u16;
    pkt.buffer.prepend_u16(cmd);

    // 加密
    encrypt_packet(pkt.buffer.data_mut(), pkt.body_size, key, pkt.client_no());

    // 1 字节序号
    let no = pkt.client_no() as u8;
    pkt.buffer.prepend_u8(no);
}

///
#[inline(always)]
fn read_robot_ws_packet(pkt: &mut NetPacketGuard) {
    // MUST only one packet in buffer

    //
    pkt.body_size = pkt.buffer_raw_len() - TO_CLIENT_HEADER_SIZE_WS;

    // 2 字节 cmd
    pkt.cmd = pkt.buffer.read_u16();
}
