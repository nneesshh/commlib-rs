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
        general_purpose::STANDARD.encode(input)
    }

    #[inline(always)]
    pub fn decode<T>(input: T) -> Result<Vec<u8>, DecodeError>
    where
        T: AsRef<[u8]>,
    {
        general_purpose::STANDARD.decode(input)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_base64() {
        use base64::{Engine as _, alphabet, engine::{self, general_purpose}};
        
        let bytes = general_purpose::STANDARD
            .decode("aGVsbG8gd29ybGR+Cg==").unwrap();
        println!("{:?}", bytes);
        
        // custom engine setup
        let bytes_url = engine::GeneralPurpose::new(
                    &alphabet::URL_SAFE,
                    general_purpose::NO_PAD)
            .decode("aGVsbG8gaW50ZXJuZXR-Cg").unwrap();
        println!("{:?}", bytes_url);
    }
}