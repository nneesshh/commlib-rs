use super::{
    net_packet::get_packet_leading_field_size, net_packet_pool::take_larget_packet,
    net_packet_pool::SMALL_PACKET_MAX_SIZE, take_packet,
};
use super::{NetPacketGuard, PacketType};

const DEFAULT_PACKET_SIZE: usize = SMALL_PACKET_MAX_SIZE;
const MAX_PACKET_SIZE: usize = 1024 * 1024 * 20; // 20M

pub enum PacketResult {
    Ready((NetPacketGuard, usize)), // (pkt, consumed)
    Suspend(usize),                 // consumed
    Abort(String),
}
///
enum PacketReaderState {
    Leading,       // 包体前导长度
    Expand(usize), // 扩展包体缓冲区（pkt_full_len）
    Data(usize),   // 包体数据区（pkt_full_len）
    Complete,      // 完成 pkt 返回给外部，内部申请新 pkt
    Abort(usize),  // 中止（pkt_full_len）
}
///
pub struct PacketReader {
    pub packet_type: PacketType,
    pub leading_field_size: usize,
    pkt_opt: Option<NetPacketGuard>, // 使用 option 以便把内部 pkt 返回给外部使用
    state: PacketReaderState,
}

impl PacketReader {
    ///
    pub fn new(packet_type: PacketType) -> PacketReader {
        let leading_field_size = get_packet_leading_field_size(packet_type);
        let pkt_opt = Some(take_packet(DEFAULT_PACKET_SIZE, packet_type));

        PacketReader {
            packet_type,
            leading_field_size,
            pkt_opt,
            state: PacketReaderState::Leading,
        }
    }

    /// Ok 返回 (pkt, consumed), Err 返回错误信息
    pub fn read(&self, input_data: *const u8, input_len: usize) -> PacketResult {
        let mut consumed = 0_usize;
        let mut remain = input_len;

        let packet_type = self.packet_type;
        let leading_field_size = self.pkt_opt.as_ref().unwrap().leading_field_size();

        while remain > 0 {
            //
            match self.state {
                PacketReaderState::Leading => {
                    let pkt_ptr: *mut NetPacketGuard = self.pkt_opt.as_ref().unwrap()
                        as *const NetPacketGuard
                        as *mut NetPacketGuard;
                    let state_ptr =
                        &self.state as *const PacketReaderState as *mut PacketReaderState;

                    let buffer_raw_len = unsafe { (*pkt_ptr).buffer_raw_len() };

                    // 包体前导长度字段是否完整？
                    if buffer_raw_len >= leading_field_size {
                        // 查看取包体前导长度
                        let pkt_full_len = unsafe { (*pkt_ptr).peek_leading_field() };
                        if pkt_full_len > MAX_PACKET_SIZE {
                            // 中止
                            unsafe {
                                (*state_ptr) = PacketReaderState::Abort(pkt_full_len);
                            }
                        } else {
                            // 检查 pkt 容量是否足够
                            let writable_bytes = unsafe { (*pkt_ptr).buffer_writable_bytes() };
                            if writable_bytes < pkt_full_len {
                                // 进入扩展包体缓冲区处理，重新申请 large pkt
                                unsafe {
                                    (*state_ptr) = PacketReaderState::Expand(pkt_full_len);
                                }
                            } else {
                                // 进入包体数据处理
                                unsafe {
                                    (*state_ptr) = PacketReaderState::Data(pkt_full_len);
                                }
                            }
                        }
                    } else {
                        let need_bytes =
                            leading_field_size - unsafe { (*pkt_ptr).buffer_raw_len() };
                        let append_bytes = if remain >= need_bytes {
                            need_bytes
                        } else {
                            remain
                        };

                        //
                        unsafe {
                            let ptr = input_data.offset(consumed as isize);
                            (*pkt_ptr).append(ptr, append_bytes);
                        }
                        remain -= append_bytes;
                        consumed += append_bytes;
                    }
                }

                PacketReaderState::Expand(pkt_full_len) => {
                    let pkt_opt_ptr: *mut Option<NetPacketGuard> = &self.pkt_opt
                        as *const Option<NetPacketGuard>
                        as *mut Option<NetPacketGuard>;
                    let pkt_ptr: *mut NetPacketGuard = self.pkt_opt.as_ref().unwrap()
                        as *const NetPacketGuard
                        as *mut NetPacketGuard;
                    let state_ptr =
                        &self.state as *const PacketReaderState as *mut PacketReaderState;

                    let buffer_raw_len = unsafe { (*pkt_ptr).buffer_raw_len() };

                    //
                    let ensure_bytes = buffer_raw_len + pkt_full_len;
                    let peek_slice = unsafe { (*pkt_ptr).peek() };

                    let new_pkt = take_larget_packet(ensure_bytes, packet_type, peek_slice);

                    //
                    unsafe {
                        (*pkt_opt_ptr) = Some(new_pkt);
                    }

                    // 进入包体数据处理
                    unsafe {
                        (*state_ptr) = PacketReaderState::Data(pkt_full_len);
                    }
                }

                PacketReaderState::Data(pkt_full_len) => {
                    let pkt_ptr: *mut NetPacketGuard = self.pkt_opt.as_ref().unwrap()
                        as *const NetPacketGuard
                        as *mut NetPacketGuard;
                    let state_ptr =
                        &self.state as *const PacketReaderState as *mut PacketReaderState;

                    let buffer_raw_len = unsafe { (*pkt_ptr).buffer_raw_len() };

                    //
                    let need_bytes = pkt_full_len - leading_field_size;

                    // 包体数据是否完整？
                    if buffer_raw_len > need_bytes {
                        std::unreachable!()
                    } else if buffer_raw_len == need_bytes {
                        // 完成当前 pkt 读取，转入完成处理
                        unsafe {
                            (*state_ptr) = PacketReaderState::Complete;
                        }
                    } else {
                        //
                        let append_bytes = if remain >= need_bytes {
                            need_bytes
                        } else {
                            remain
                        };

                        //
                        unsafe {
                            let ptr = input_data.offset(consumed as isize);
                            (*pkt_ptr).append(ptr, append_bytes);
                        }
                        remain -= append_bytes;
                        consumed += append_bytes;
                    }
                }

                PacketReaderState::Complete => {
                    let pkt_opt_ptr: *mut Option<NetPacketGuard> = &self.pkt_opt
                        as *const Option<NetPacketGuard>
                        as *mut Option<NetPacketGuard>;
                    /*let pkt_ptr: *mut NetPacketGuard = self.pkt_opt.as_ref().unwrap()
                    as *const NetPacketGuard
                    as *mut NetPacketGuard;*/
                    let state_ptr =
                        &self.state as *const PacketReaderState as *mut PacketReaderState;

                    // 完成当前 pkt 读取，重新开始包体前导长度处理，并把剩余长度返回外部
                    unsafe {
                        (*state_ptr) = PacketReaderState::Leading;
                    }

                    // 将完整的 pkt 返回到外部使用，内部补充新的 pkt
                    let new_pkt = take_packet(DEFAULT_PACKET_SIZE, packet_type);
                    unsafe {
                        let old_pkt = (*pkt_opt_ptr).take().unwrap();
                        (*pkt_opt_ptr) = Some(new_pkt);
                        return PacketResult::Ready((old_pkt, consumed));
                    }
                }

                PacketReaderState::Abort(pkt_full_len) => {
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
