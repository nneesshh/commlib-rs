#include "crypto_bindings.h"

#include "crypto/blowfish.h"

namespace commlib
{
	BlowfishCfb64::BlowfishCfb64(): m_inner_bf(), m_inner(m_inner_bf) {

	}

	void BlowfishCfb64::setKey(rust::Slice<const uint8_t> key) {
		m_inner_bf.set_key((const char*)key.data(), key.length());
	}

	void BlowfishCfb64::setInitVec(const uint64_t init_vec) {
		m_inner.set_init_vector(init_vec);
	}

	rust::Vec<uint8_t> BlowfishCfb64::encrypt(rust::Slice<const uint8_t> data) {
		rust::Vec<uint8_t> v;
		v.reserve(data.size());
		std::copy(data.begin(), data.end(), std::back_inserter(v));
		m_inner.encrypt((unsigned char*)v.data(), v.size());
		return v;
	}

	rust::Vec<uint8_t> BlowfishCfb64::decrypt(rust::Slice<const uint8_t> data) {
		rust::Vec<uint8_t> v;
		v.reserve(data.size());
		std::copy(data.begin(), data.end(), std::back_inserter(v));
		m_inner.decrypt((unsigned char*)v.data(), v.size());
		return v;
	}

}
