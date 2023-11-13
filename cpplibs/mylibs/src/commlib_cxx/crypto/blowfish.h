#ifndef BLOWFISH_H
#define	BLOWFISH_H

#include <new>
#include <string>
#include <cstddef>

class Blowfish
{
  public:
    // @throws std::bad_alloc
    Blowfish();

    // @throws std::bad_alloc
    Blowfish(const Blowfish& other);
    Blowfish& operator=(const Blowfish& other);
    Blowfish(Blowfish&& orig);
    Blowfish& operator=(Blowfish&& orig);

    virtual ~Blowfish() noexcept;

    /**
     * Reinitializes the cipher instance, so that a new key can be set, and the cipher can be reused
     */
    virtual void reinitialize() noexcept;

    /**
     * Clears the cipher instance's state by zeroing out the S-boxes and P-boxes
     */
    virtual void clear() volatile noexcept;

    /**
     * Returns the cipher data for a single data block of plain text
     *
     * @param data The plain data to encrypt
     * @return The cipher data for the supplied plain data
     */
    virtual uint64_t encrypt64(uint64_t data) noexcept;

    /**
     * Returns the plain data for a single data block of cipher text
     * @param data The cipher data to decrypt
     * @return The plain data for the supplied cipher data
     */
    virtual uint64_t decrypt64(uint64_t data) noexcept;

    /**
     * Encrypts the two 32 bit parts of a single 64 bit block of data
     *
     * @param data_l_ref The left (first, big-endian high-order) 32 bits of data
     * @param data_r_ref The right (second, big-endian low-order) 32 bits of data
     */
    virtual void encrypt(uint32_t* data_l_ref, uint32_t* data_r_ref) noexcept;

    /**
     * Decrypts the two 32 bit parts of a single 64 bit block of data
     *
     * @param data_l_ref The left (first, big-endian high-order) 32 bits of data
     * @param data_r_ref The right (second, big-endian low-order) 32 bits of data
     */
    virtual void decrypt(uint32_t* data_l_ref, uint32_t* data_r_ref) noexcept;

    /**
     * Sets the encryption key by applying the key to the cipher's S-boxes and P-boxes
     *
     * @param key The encryption key to apply to the cipher instance
     */
    virtual void set_key(const std::string& key) noexcept;

    /**
     * Sets the encryption key by applying the key to the cipher's S-boxes and P-boxes
     *
     * @param key The encryption key to apply to the cipher instance
     * @param key_length The length of the encryption key
     */
    virtual void set_key(const char* const key, const size_t key_length) noexcept;

  private:
    struct bf_state
    {
        volatile uint32_t p_box[18];
        volatile uint32_t s_box[4][256];
    };

    bf_state* state {nullptr};
    bool is_clear {false};

    static const bf_state INIT_STATE;
    static const size_t   P_BOXES;
    static const size_t   S_BOXES;
    static const size_t   S_BOX_ENTRIES;

    // Cipher rounds
    static const size_t ROUNDS;

    // Step width for the unrolled loops
    static const size_t UNROLLED_STEP;

    inline uint32_t blowfish_f(uint32_t value) noexcept;
    inline void copy_bf_state(const bf_state& src_state) volatile noexcept;
};

#endif // BLOWFISH_H
