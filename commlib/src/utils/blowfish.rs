//!
//! Commlib: Blowfish
//!

use commlib_sys::crypto::{
    blowfish_decrypt, blowfish_encrypt, blowfish_set_init_vec, blowfish_set_key, new_blowfish,
};
pub struct Blowfish();

impl Blowfish {
    ///
    pub fn encrypt(key: &[u8], iv: u64, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = new_blowfish();
        blowfish_set_key(cipher.clone(), key);
        blowfish_set_init_vec(cipher.clone(), iv);

        let ciphertext = blowfish_encrypt(cipher.clone(), plaintext);
        Ok(ciphertext)
    }

    ///
    pub fn decrypt(key: &[u8], iv: u64, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = new_blowfish();
        blowfish_set_key(cipher.clone(), key);
        blowfish_set_init_vec(cipher.clone(), iv);

        let plaintext = blowfish_decrypt(cipher.clone(), ciphertext);
        Ok(plaintext)
    }
}

/*
use openssl::symm::{decrypt as ossl_decrypt, encrypt as ossl_encrtypt, Cipher};

const IV: &[u8] = &[0_u8; 8];

pub struct Blowfish();

impl Blowfish {
    ///
    pub fn encrypt(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Cipher::bf_cfb64();

        let ciphertext_opt = ossl_encrtypt(cipher, key, Some(IV), plaintext);
        match ciphertext_opt {
            Ok(r) => Ok(r),
            Err(error) => {
                log::error!(
                    "Blowfish encrypt error: {:?}, key: {:?} plaintext: {:?}",
                    error,
                    key,
                    plaintext
                );
                Err(error.to_string())
            }
        }
    }

    ///
    pub fn decrypt(key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Cipher::bf_cfb64();

        let plaintext_opt = ossl_decrypt(cipher, key.as_ref(), Some(IV), ciphertext.as_ref());
        match plaintext_opt {
            Ok(r) => Ok(r),
            Err(error) => {
                log::error!(
                    "Blowfish decrypt error: {:?}, key: {:?} ciphertext: {:?}",
                    error,
                    key,
                    ciphertext
                );
                Err(error.to_string())
            }
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use hex::{self, FromHex};

    use crate::utils::Blowfish;

    #[test]
    pub fn test_bf_cfb64() {
        let pt = "37363534333231204E6F77206973207468652074696D6520666F722000";
        let ct = "E73214A2822139CAF26ECF6D2EB9E76E3DA3DE04D1517200519D57A6C3";
        let key = "0123456789ABCDEFF0E1D2C3B4A59687";
        let iv = "FEDCBA9876543210";

        let pt = Vec::from_hex(pt).unwrap();
        let ct = Vec::from_hex(ct).unwrap();
        let key = Vec::from_hex(key).unwrap();
        let iv = Vec::from_hex(iv).unwrap();

        let slice: [u8; 8] = iv.as_slice().try_into().unwrap();
        let iv = u64::from_be_bytes(slice);

        let plaintext = Blowfish::decrypt(key.as_slice(), iv, ct.as_slice()).unwrap();
        let ciphertext = Blowfish::encrypt(key.as_slice(), iv, pt.as_slice()).unwrap();
        assert_eq!(plaintext.as_slice(), pt.as_slice());
        assert_eq!(ciphertext.as_slice(), ct.as_slice());
    }
}
