include!("ffi_main.rs");

use cxx::{type_id, ExternType};

#[repr(transparent)]
pub struct SignalCallback(pub extern "C" fn(sig: i32));

unsafe impl ExternType for SignalCallback {
    type Id = type_id!("SignalCallback");
    type Kind = cxx::kind::Trivial;
}
