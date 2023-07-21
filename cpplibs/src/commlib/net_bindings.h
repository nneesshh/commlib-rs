#ifndef __NET_BINDINGS_H__
#define __NET_BINDINGS_H__

#include <memory>
#include "rust/cxx.h"
#include "rust/net.rs.h"
#include "service_net.h"

struct UserService;

namespace commlib
{
    std::unique_ptr<ServiceNet> service_net_new(int32_t n) {
        return std::make_unique<ServiceNet>(n);
    }


} // namespace commlib

#endif // __NET_BINDINGS_H__
