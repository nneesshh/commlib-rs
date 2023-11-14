#include "blowfish_cfb64.h"

CBlowfishCfb64::CBlowfishCfb64(CBlowfish& cipher_ref):
    cipher(&cipher_ref)
{
}

CBlowfishCfb64::~CBlowfishCfb64()
{
    feedback = 0;
}

void CBlowfishCfb64::encrypt(unsigned char* const data, const size_t data_length)
{
    uint64_t cipher_text = feedback;
    size_t full_blocks = data_length / BLOCK_SIZE;
    for (size_t block_index = 0; block_index < full_blocks; ++block_index)
    {
        cipher_text = cipher->encrypt64(cipher_text);

        uint64_t plain_text = 0;
        // Get the plain text from the data string
        for (size_t offset = 0; offset < BLOCK_SIZE; ++offset)
        {
            size_t data_index = block_index * BLOCK_SIZE + offset;
            plain_text = plain_text << BYTE_SHIFT;
            plain_text |= data[data_index];
        }

        // XOR cipher text and plain text
        cipher_text = cipher_text ^ plain_text;

        // Write the cipher text back to the data string
        for (size_t offset = 0; offset < BLOCK_SIZE; ++offset)
        {
            size_t data_index = block_index * BLOCK_SIZE + offset;
            data[data_index] = static_cast<unsigned char> (
                (cipher_text >> ((REMAINDER_BASE - offset) * BYTE_SHIFT)) & BYTE_MASK
            );
        }
    }

    size_t remainder = data_length % BLOCK_SIZE;
    if (remainder > 0)
    {
        cipher_text = cipher->encrypt64(cipher_text);

        uint64_t plain_text = 0;
        // Get the remainder of the plain text from the data string
        for (size_t offset = 0; offset < remainder; ++offset)
        {
            size_t data_index = data_length - remainder + offset;
            plain_text = plain_text << BYTE_SHIFT;
            plain_text |= static_cast<uint64_t> (data[data_index]);
        }
        // Finish the shift to the left
        plain_text = plain_text << ((BLOCK_SIZE - remainder) * BYTE_SHIFT);

        cipher_text = cipher_text ^ plain_text;

        // Write the remainder of the cipher text back to the data string
        for (size_t offset = 0; offset < remainder; ++offset)
        {
            size_t data_index = data_length - remainder + offset;
            data[data_index] = static_cast<unsigned char> (
                (cipher_text >> ((REMAINDER_BASE - offset) * BYTE_SHIFT)) & BYTE_MASK
            );
        }
    }

    feedback = cipher_text;
}

void CBlowfishCfb64::decrypt(unsigned char* const data, const size_t data_length)
{
    uint64_t cipher_base = feedback;

    size_t full_blocks = data_length / BLOCK_SIZE;
    for (size_t block_index = 0; block_index < full_blocks; ++block_index)
    {
        // Encrypt the current block
        cipher_base = cipher->encrypt64(cipher_base);

        uint64_t cipher_text = 0;
        // Get the cipher text from the data string
        for (size_t offset = 0; offset < BLOCK_SIZE; ++offset)
        {
            size_t data_index = block_index * BLOCK_SIZE + offset;
            cipher_text = cipher_text << BYTE_SHIFT;
            cipher_text |= data[data_index];
        }

        // Decrypt the block
        const uint64_t plain_text = cipher_text ^ cipher_base;

        // Write the plain text back to the data string
        for (size_t offset = 0; offset < BLOCK_SIZE; ++offset)
        {
            size_t data_index = block_index * BLOCK_SIZE + offset;
            data[data_index] = static_cast<unsigned char> (
                (plain_text >> ((REMAINDER_BASE - offset) * BYTE_SHIFT)) & BYTE_MASK
            );
        }

        // Set the cipher input for the next block
        cipher_base = cipher_text;
    }

    size_t remainder = data_length % BLOCK_SIZE;
    if (remainder > 0)
    {
        cipher_base = cipher->encrypt64(cipher_base);

        uint64_t cipher_text = 0;
        for (size_t offset = 0; offset < remainder; ++offset)
        {
            size_t data_index = data_length - remainder + offset;
            cipher_text = cipher_text << BYTE_SHIFT;
            cipher_text |= data[data_index];
        }
        // Finish the shift to the left
        cipher_text = cipher_text << ((BLOCK_SIZE - remainder) * BYTE_SHIFT);

        // Decrypt the block
        const uint64_t plain_text = cipher_text ^ cipher_base;

        // Write the remainder of the plain text back to the data string
        for (size_t offset = 0; offset < remainder; ++offset)
        {
            size_t data_index = data_length - remainder + offset;
            data[data_index] = static_cast<unsigned char> (
                (plain_text >> ((REMAINDER_BASE - offset) * BYTE_SHIFT)) & BYTE_MASK
            );
        }
    }

    feedback = cipher_base;
}

void CBlowfishCfb64::set_init_vector(const uint64_t init_vector)
{
    feedback = init_vector;
}
