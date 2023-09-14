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

#[cfg(test)]
mod test {
    use hex::FromHex;
    use openssl::symm::{Cipher, Crypter, Mode};

    use crate::{Blowfish, PlayerId};

    #[test]
    fn test_bf_cfb64() {
        crate::ossl_init();

        let pt = "37363534333231204E6F77206973207468652074696D6520666F722000";
        let ct = "E73214A2822139CAF26ECF6D2EB9E76E3DA3DE04D1517200519D57A6C3";
        let key = "0123456789ABCDEFF0E1D2C3B4A59687";
        let iv = "FEDCBA9876543210";

        cipher_test_nopad(Cipher::bf_cfb64(), pt, ct, key, iv);

        let ciphertext = Blowfish::encrypt(key.as_bytes(), pt.as_bytes()).unwrap();
        let plaintext = Blowfish::decrypt(key.as_bytes(), &ciphertext.as_slice()).unwrap();
        assert_eq!(plaintext.as_slice(), pt.as_bytes());
    }

    fn cipher_test_nopad(ciphertype: Cipher, pt: &str, ct: &str, key: &str, iv: &str) {
        let pt = Vec::from_hex(pt).unwrap();
        let ct = Vec::from_hex(ct).unwrap();
        let key = Vec::from_hex(key).unwrap();
        let iv = Vec::from_hex(iv).unwrap();

        let computed = {
            let mut c = Crypter::new(ciphertype, Mode::Decrypt, &key, Some(&iv)).unwrap();
            c.pad(false);
            let mut out = vec![0; ct.len() + ciphertype.block_size()];
            let count = c.update(&ct, &mut out).unwrap();
            let rest = c.finalize(&mut out[count..]).unwrap();
            out.truncate(count + rest);
            out
        };
        let expected = pt;

        if computed != expected {
            println!("Computed: {}", hex::encode(&computed));
            println!("Expected: {}", hex::encode(&expected));
            if computed.len() != expected.len() {
                println!(
                    "Lengths differ: {} in computed vs {} expected",
                    computed.len(),
                    expected.len()
                );
            }
            panic!("test failure");
        }
    }
}
