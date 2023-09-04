use super::NetPacketGuard;
use super::{take_large_packet, take_packet};

const MAX_PACKET_SIZE: usize = 1024 * 1024 * 20; // 20M

/// Read result
pub enum PacketResult {
    Ready(Vec<NetPacketGuard>), // pkt list
    Abort(String),
}

/// Reader state
enum PacketReceiverState {
    Null,                   // 空包
    Leading,                // 包体前导长度
    Expand(usize),          // 扩展包体缓冲区（pkt_full_len）
    Data(usize),            // 包体数据区（pkt_full_len）
    Complete,               // 完成一个 pkt
    CompleteTailing(usize), // 完成一个 pkt，尾部冗余（pkt_full_len），申请新 pkt 容纳尾部多余数据
    Abort(usize),           // 中止（pkt_full_len）
}

///
pub struct PacketReceiver {
    pkt_opt: Option<NetPacketGuard>, // 使用 option 以便 pkt 移交
    state: PacketReceiverState,
}

impl PacketReceiver {
    ///
    pub fn new() -> PacketReceiver {
        PacketReceiver {
            pkt_opt: None,
            state: PacketReceiverState::Null,
        }
    }

    /// buffer_pkt 中存放 input 数据，一次性处理完毕，Ok 返回 pkt_list, Err 返回错误信息
    pub fn read(&self, buffer_pkt: NetPacketGuard) -> PacketResult {
        //
        let mut pkt_list = Vec::new();

        let mut consumed = 0_usize;
        let mut remain = buffer_pkt.buffer_raw_len();

        let leading_field_size = buffer_pkt.leading_field_size();

        // debug only
        {
            let input = buffer_pkt.peek();
            log::info!("input: {:?}", input);
            let input_hex = hex::encode(input);
            log::info!("input_hex: {}", input_hex);
        }

        // option trick 以便 pkt 移交
        let mut input_pkt_opt = Some(buffer_pkt);

        //
        let pkt_opt_ptr_: *mut Option<NetPacketGuard> =
            &self.pkt_opt as *const Option<NetPacketGuard> as *mut Option<NetPacketGuard>;
        let state_ptr_ = &self.state as *const PacketReceiverState as *mut PacketReceiverState;

        let pkt_opt = unsafe { &mut *pkt_opt_ptr_ };
        let state = unsafe { &mut *state_ptr_ };

        loop {
            //
            match self.state {
                PacketReceiverState::Null => {
                    let input_pkt = input_pkt_opt.take().unwrap();
                    let append_bytes = input_pkt.buffer_raw_len();

                    // 初始化 pkt，直接使用 input_pkt 避免 copy
                    (*pkt_opt) = Some(input_pkt);

                    // state: 进入包体前导长度读取
                    (*state) = PacketReceiverState::Leading;

                    //
                    remain -= append_bytes;
                    consumed += append_bytes;
                }

                PacketReceiverState::Leading => {
                    let pkt = pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体前导长度字段是否完整？
                    if buffer_raw_len >= leading_field_size as usize {
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
                    } else if remain > 0 {
                        // buffer_pkt 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                        let need_bytes = leading_field_size as usize - buffer_raw_len;
                        let append_bytes = if remain >= need_bytes {
                            need_bytes
                        } else {
                            remain
                        };

                        // 消耗 buffer_pkt 中的 input 数据
                        let mut input_pkt = input_pkt_opt.take().unwrap();
                        let to_append = input_pkt.consume_n(append_bytes);
                        pkt.append_slice(to_append);

                        //
                        remain -= append_bytes;
                        consumed += append_bytes;

                        // not enough for append (输入数据已经消耗完毕)
                        if append_bytes < need_bytes {
                            break;
                        }
                    } else {
                        // not enough for append (输入数据已经消耗完毕)
                        break;
                    }
                }

                PacketReceiverState::Expand(pkt_full_len) => {
                    let old_pkt = pkt_opt.as_mut().unwrap();

                    //
                    let ensure_bytes = pkt_full_len;
                    let old_pkt_slice = old_pkt.consume();

                    // old pkt 数据转移到 new pkt，使用 new pkt 继续解析
                    let new_pkt =
                        take_large_packet(leading_field_size, ensure_bytes, old_pkt_slice);
                    (*pkt_opt) = Some(new_pkt);

                    // 进入包体数据处理
                    (*state) = PacketReceiverState::Data(pkt_full_len);
                }

                PacketReceiverState::Data(pkt_full_len) => {
                    let pkt = pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体数据是否完整？
                    if buffer_raw_len > pkt_full_len {
                        // state: 完成当前 pkt 读取，转入完成处理（pkt尾部有多余数据）
                        (*state) = PacketReceiverState::CompleteTailing(pkt_full_len);
                    } else if buffer_raw_len == pkt_full_len {
                        // state: 完成当前 pkt 读取，转入完成处理
                        (*state) = PacketReceiverState::Complete;
                    } else if remain > 0 {
                        // buffer_pkt 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                        assert!(buffer_raw_len + consumed <= pkt_full_len);
                        let need_types = pkt_full_len - (buffer_raw_len + consumed);
                        let append_bytes = if remain >= need_types {
                            need_types
                        } else {
                            remain
                        };

                        // 消耗 input_pkt 中的 input 数据
                        let input_pkt = input_pkt_opt.as_mut().unwrap();
                        let to_append = input_pkt.consume_n(append_bytes);
                        pkt.append_slice(to_append);

                        //
                        remain -= append_bytes;
                        consumed += append_bytes;

                        // not enough for append (输入数据已经消耗完毕)
                        if append_bytes < need_types {
                            break;
                        }
                    } else {
                        // not enough for append (输入数据已经消耗完毕)
                        break;
                    }
                }

                PacketReceiverState::Complete => {
                    // 完成一个 pkt
                    let ready_pkt = pkt_opt.take().unwrap();
                    (*pkt_opt) = None;

                    //
                    pkt_list.push(ready_pkt);

                    // 完成当前 pkt 读取，进入空包状态
                    (*state) = PacketReceiverState::Null;

                    // input 数据处理完毕
                    if 0 == remain {
                        break;
                    }
                    assert!(input_pkt_opt.is_some());
                }

                PacketReceiverState::CompleteTailing(pkt_full_len) => {
                    // 截取多余的尾部，创建新包
                    let pkt = pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();
                    assert!(pkt_full_len < buffer_raw_len);

                    let tail_len = buffer_raw_len - pkt_full_len;

                    // 复制字节数较少的部分
                    if pkt_full_len < tail_len {
                        // 完成包较小，使用新包进行容纳
                        let mut ready_pkt = take_packet(pkt_full_len, leading_field_size);
                        {
                            let to_append = pkt.consume_n(pkt_full_len);
                            ready_pkt.append_slice(to_append);
                        }

                        //
                        pkt_list.push(ready_pkt);
                    } else {
                        // 尾部较小，使用新包进行容纳
                        let mut ready_pkt = pkt_opt.take().unwrap();
                        {
                            let to_append = ready_pkt.consume_tail_n(tail_len);

                            //
                            let mut tail_pkt = take_packet(pkt_full_len, leading_field_size);
                            tail_pkt.append_slice(to_append);

                            (*pkt_opt) = Some(tail_pkt);
                        }

                        //
                        pkt_list.push(ready_pkt);
                    }

                    // 完成当前 pkt 读取，重新开始包体前导长度处理
                    (*state) = PacketReceiverState::Leading;
                }

                PacketReceiverState::Abort(pkt_full_len) => {
                    log::error!("packet overflow!!! pkt_full_len={}", pkt_full_len);

                    // 包长度越界
                    return PacketResult::Abort("overflow".to_owned());
                }
            }
        }

        // 完成包列表
        PacketResult::Ready(pkt_list)
    }
}
