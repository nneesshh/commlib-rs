#ifndef BLOWFISH_CFB64_H
#define	BLOWFISH_CFB64_H

#include <cstddef>
#include "blowfish.h"

class CBlowfishCfb64
{
  public:
    // Block size in bytes (8 == 64 bits)
    static const size_t BLOCK_SIZE = 8;

    // Maximum shift width for the remaining data in bytes (7 == 56 bits)
    static const size_t REMAINDER_BASE = 7;

    // Byte mask, used to extract a single byte from a bigger datatype
    static const uint64_t BYTE_MASK = 0xFF;

    // Byte shift value (8 bits == 1 byte)
    static const size_t BYTE_SHIFT = 8;

    CBlowfishCfb64(CBlowfish& cipher_ref);
    CBlowfishCfb64(const CBlowfishCfb64& orig) = default;
    CBlowfishCfb64& operator=(const CBlowfishCfb64& orig) = default;
    CBlowfishCfb64(CBlowfishCfb64&& orig) = default;
    CBlowfishCfb64& operator=(CBlowfishCfb64&& orig) = default;
    virtual ~CBlowfishCfb64();

    /**
     * Encrypts the supplied data in-place
     *
     * @param data Plain text input data to encrypt
     * @param data_length Length of the input data
     */
    virtual void encrypt(unsigned char* data, size_t data_length);

    /**
     * Decrypts the supplied data in-place
     *
     * @param data Cipher text input data to decrypt
     * @param data_length Length of the input data
     */
    virtual void decrypt(unsigned char* data, size_t data_length);

    /**
     * Sets the initialization vector
     *
     * @param init_vector Initialization vector for the CFB stream cipher
     */
    virtual void set_init_vector(uint64_t init_vector);

  private:
    uint64_t  feedback {0};
    CBlowfish* cipher;
};

#endif	/* BLOWFISH_CFB64_H */
