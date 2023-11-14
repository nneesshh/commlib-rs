#ifndef __HASH_BINDINGS_H__
#define __HASH_BINDINGS_H__

#include "rust/cxx.h"

namespace commlib
{
    rust::String md5(rust::Slice<const uint8_t> data);
    size_t md5_block_size();
    size_t md5_hash_bytes();

} // namespace commlib

#endif // __HASH_BINDINGS_H__
