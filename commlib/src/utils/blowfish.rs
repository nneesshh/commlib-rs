//! Commlib: Blowfish

use openssl::symm::{decrypt as ossl_decrypt, encrypt as ossl_encrtypt, Cipher};

/* initialization vector */
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
