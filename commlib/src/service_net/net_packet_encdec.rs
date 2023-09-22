use crate::rand_between_exclusive_i8;

use super::PKT_CMD_LEN;
use super::{ConnId, EncryptData, NetPacketGuard, PacketType};

/// 消息体加密字节数
const ENCRYPT_MAX_BODY_LEN: usize = 4; /* 4字节消息体 */
pub const ENCRYPT_MAX_LEN: usize = PKT_CMD_LEN + ENCRYPT_MAX_BODY_LEN;
pub const ENCRYPT_KEY_LEN: usize = 64; /* 密钥总长度，根据 client no 进行偏移 */
const SAVED_NO_COUNT: usize = 1;

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
                    pkt.read_client_packet(encrypt.encrypt_key.as_str());

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
                    pkt.read_client_ws_packet(encrypt.encrypt_key.as_str());

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
            pkt.read_server_packet();
            true
        }

        PacketType::Robot => {
            pkt.read_robot_packet();
            true
        }

        PacketType::RobotWs => {
            pkt.read_robot_ws_packet();
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
                pkt.write_robot_packet(encrypt.encrypt_key.as_str());

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
                pkt.write_robot_ws_packet(encrypt.encrypt_key.as_str());

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
            pkt.write_server_packet();
            true
        }

        PacketType::Client => {
            pkt.write_client_packet();
            true
        }

        PacketType::ClientWs => {
            pkt.write_client_ws_packet();
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
