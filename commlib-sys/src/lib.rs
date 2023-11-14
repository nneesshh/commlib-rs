//! Commlib-sys: crypto, hash, signal, ...

include!("../ffi/crypto.rs");
include!("../ffi/hash.rs");
include!("../ffi/signal.rs");

pub mod crypto {
    pub use crate::ffi_crypto::{
        blowfish_decrypt, blowfish_encrypt, blowfish_set_init_vec, blowfish_set_key, new_blowfish,
    };
}

pub mod hash {
    pub use crate::ffi_hash::md5;
}

pub mod sig {
    pub use crate::ffi_sig::init_signal_handlers;
}
