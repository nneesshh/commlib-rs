//! Commlib: Base64
use base64::{engine::general_purpose, DecodeError, Engine as _};

pub struct Base64();

impl Base64 {
    ///
    #[inline(always)]
    pub fn encode<T>(input: T) -> String
    where
        T: AsRef<[u8]>,
    {
        general_purpose::URL_SAFE_NO_PAD.encode(input)
    }

    #[inline(always)]
    pub fn decode<T>(input: T) -> Result<Vec<u8>, DecodeError>
    where
        T: AsRef<[u8]>,
    {
        general_purpose::URL_SAFE_NO_PAD.decode(input)
    }
}
