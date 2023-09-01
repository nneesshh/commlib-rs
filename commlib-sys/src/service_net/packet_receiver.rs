use super::{
    net_packet::get_packet_leading_field_size,
    net_packet_pool::{take_large_packet, take_small_packet},
};
use super::{NetPacketGuard, PacketType};

const MAX_PACKET_SIZE: usize = 1024 * 1024 * 20; // 20M

/// Read result
pub enum PacketResult {
    Ready((NetPacketGuard, usize)), // (pkt, consumed)
    Suspend(usize),                 // consumed
    Abort(String),
}

/// Reader state
enum PacketReceiverState {
    Leading,       // 包体前导长度
    Expand(usize), // 扩展包体缓冲区（pkt_full_len）
    Data(usize),   // 包体数据区（pkt_full_len）
    Complete,      // 完成 pkt 返回给外部，内部申请新 pkt
    Abort(usize),  // 中止（pkt_full_len）
}

///
pub struct PacketReceiver {
    pub leading_field_size: usize,
    pkt_opt: Option<NetPacketGuard>, // 使用 option 以便把内部 pkt 返回给外部使用
    state: PacketReceiverState,
}

impl PacketReceiver {
    ///
    pub fn new(pkt: NetPacketGuard) -> PacketReceiver {
        let leading_field_size = pkt.leading_field_size();
        let pkt_opt = Some(pkt);

        PacketReceiver {
            leading_field_size,
            pkt_opt,
            state: PacketReceiverState::Leading,
        }
    }

    /// Ok 返回 (pkt, consumed), Err 返回错误信息
    pub fn read(&self, input_data: *const u8, input_len: usize) -> PacketResult {
        let mut consumed = 0_usize;
        let mut remain = input_len;

        let leading_field_size = self.leading_field_size;

        let input = unsafe { std::slice::from_raw_parts(input_data, input_len) };
        log::info!("input: {:?}", input);
        let input_hex = hex::encode(input);
        log::info!("input_hex: {}", input_hex);

        let pkt_opt_ptr_: *mut Option<NetPacketGuard> =
            &self.pkt_opt as *const Option<NetPacketGuard> as *mut Option<NetPacketGuard>;
        let state_ptr_ = &self.state as *const PacketReceiverState as *mut PacketReceiverState;

        let pkt_opt = unsafe { &mut *pkt_opt_ptr_ };
        let state = unsafe { &mut *state_ptr_ };

        loop {
            //
            match self.state {
                PacketReceiverState::Leading => {
                    let pkt = pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体前导长度字段是否完整？
                    if buffer_raw_len >= leading_field_size {
                        // 查看取包体前导长度
                        let pkt_full_len = pkt.peek_leading_field();
                        if pkt_full_len > MAX_PACKET_SIZE {
                            // state: 中止
                            (*state) = PacketReceiverState::Abort(pkt_full_len);
                        } else {
                            // 检查 pkt 容量是否足够
                            let writable_bytes = pkt.buffer_writable_bytes();
                            if writable_bytes < pkt_full_len {
                                // state: 进入扩展包体缓冲区处理，重新申请 large pkt
                                (*state) = PacketReceiverState::Expand(pkt_full_len);
                            } else {
                                // state: 进入包体数据处理
                                (*state) = PacketReceiverState::Data(pkt_full_len);
                            }
                        }
                    } else {
                        let need_bytes = leading_field_size - pkt.buffer_raw_len();
                        let append_bytes = if remain >= need_bytes {
                            need_bytes
                        } else {
                            remain
                        };

                        //
                        unsafe {
                            let ptr = input_data.offset(consumed as isize);
                            pkt.append(ptr, append_bytes);
                        }
                        remain -= append_bytes;
                        consumed += append_bytes;

                        // enough for append?
                        if append_bytes < need_bytes {
                            break;
                        }
                    }
                }

                PacketReceiverState::Expand(pkt_full_len) => {
                    let old_pkt = pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = old_pkt.buffer_raw_len();

                    //
                    let ensure_bytes = buffer_raw_len + pkt_full_len;
                    let old_pkt_slice = old_pkt.peek();

                    // old pkt 数据转移到 new pkt，使用 new pkt 继续解析
                    let new_pkt = take_large_packet(ensure_bytes, old_pkt_slice);
                    (*pkt_opt) = Some(new_pkt);

                    // 进入包体数据处理
                    (*state) = PacketReceiverState::Data(pkt_full_len);
                }

                PacketReceiverState::Data(pkt_full_len) => {
                    let pkt = pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体数据是否完整？
                    if buffer_raw_len > pkt_full_len {
                        std::unreachable!()
                    } else if buffer_raw_len == pkt_full_len {
                        // state: 完成当前 pkt 读取，转入完成处理
                        (*state) = PacketReceiverState::Complete;
                    } else {
                        //
                        assert!(buffer_raw_len + consumed <= pkt_full_len);
                        let need_types = pkt_full_len - (buffer_raw_len + consumed);
                        let append_bytes = if remain >= need_types {
                            need_types
                        } else {
                            remain
                        };

                        //
                        unsafe {
                            let ptr = input_data.offset(consumed as isize);
                            pkt.append(ptr, append_bytes);
                        }
                        remain -= append_bytes;
                        consumed += append_bytes;

                        // enough for append?
                        if append_bytes < need_types {
                            break;
                        }
                    }
                }

                PacketReceiverState::Complete => {
                    // 将完整的 pkt 返回到外部使用，内部补充新的 pkt
                    let new_pkt = take_small_packet();
                    let old_pkt = pkt_opt.take().unwrap();
                    (*pkt_opt) = Some(new_pkt);

                    // 完成当前 pkt 读取，重新开始包体前导长度处理，并把剩余长度返回外部
                    (*state) = PacketReceiverState::Leading;

                    //
                    return PacketResult::Ready((old_pkt, consumed));
                }

                PacketReceiverState::Abort(pkt_full_len) => {
                    log::error!("packet overflow!!! pkt_full_len={}", pkt_full_len);

                    // 包长度越界
                    return PacketResult::Abort("overflow".to_owned());
                }
            }
        }

        // 包体不完整
        PacketResult::Suspend(consumed)
    }
}
