#include "hash_bindings.h"
#include "hash/md5.h"

namespace commlib
{
	rust::String md5(rust::Slice<const uint8_t> data)
	{
		MD5 md5;
		return md5(data.data(), data.length());
	}

	size_t md5_block_size()
	{
		return MD5::blockSize();
	}

	size_t md5_hash_bytes()
	{
		return MD5::hashBytes();
	}
}
