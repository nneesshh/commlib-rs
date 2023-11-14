#ifndef __CRYPTO_BINDINGS_H__
#define __CRYPTO_BINDINGS_H__

#include <memory>
#include "rust/cxx.h"
#include "crypto/blowfish_cfb64.h"

namespace commlib
{
    class BlowfishCfb64 {
    public:
        BlowfishCfb64();

        void setKey(rust::Slice<const uint8_t> key);
        void setInitVec(const uint64_t init_vec);

        rust::Vec<uint8_t> encrypt(rust::Slice<const uint8_t> data);
        rust::Vec<uint8_t> decrypt(rust::Slice<const uint8_t> data);

    private:
        ::CBlowfish m_inner_bf;
        ::CBlowfishCfb64 m_inner;
    };

	//// Blowfish
	std::shared_ptr<BlowfishCfb64> new_blowfish();
	void blowfish_set_key(std::shared_ptr<BlowfishCfb64> bf, rust::Slice<const uint8_t> key);
    void blowfish_set_init_vec(std::shared_ptr<BlowfishCfb64> bf, uint64_t init_vec);
    rust::Vec<uint8_t> blowfish_encrypt(std::shared_ptr<BlowfishCfb64> bf, rust::Slice<const uint8_t> data);
    rust::Vec<uint8_t> blowfish_decrypt(std::shared_ptr<BlowfishCfb64> bf, rust::Slice<const uint8_t> data);

} // namespace commlib

#endif // __CRYPTO_BINDINGS_H__
