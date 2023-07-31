#ifndef __NET_BINDINGS_H__
#define __NET_BINDINGS_H__

#include <memory>
#include <functional>
#include "rust/cxx.h"
#include "rust/common.rs.h"
#include "service_net.h"

namespace commlib {
    namespace evpp {
        /// <summary>
        /// 
        /// </summary>
        class TCPConn {
            virtual ~TCPConn() {}
        };
    }
}

////
struct ServiceWrapper;

/// <summary>
/// 
/// </summary>
using OnTcpListen = void(*)(ServiceWrapper& srv, rust::String name);
using OnTcpAccept = void(*)(ServiceWrapper& srv, std::shared_ptr<commlib::evpp::TCPConn> conn);
using OnTcpEncrpty = void(*)(ServiceWrapper& srv, std::shared_ptr<commlib::evpp::TCPConn> conn);

using OnTcpConnect = void(*)(ServiceWrapper& srv, std::shared_ptr<commlib::evpp::TCPConn> conn);
using OnTcpPacket = void(*)(ServiceWrapper& srv, std::shared_ptr<commlib::evpp::TCPConn> conn, commlib::NetPacket* pkt);
using OnTcpClose = void(*)(ServiceWrapper& srv, std::shared_ptr<commlib::evpp::TCPConn> conn);

/// <summary>
/// 
/// </summary>
struct TcpCallbacks {
    OnTcpListen on_listen;
    OnTcpAccept on_accept;
    OnTcpEncrpty on_encrypt;

    OnTcpConnect on_connect;
    OnTcpPacket on_packet;
    OnTcpClose on_close;
};

namespace commlib
{
    // std::unique_ptr<ServiceNet> service_net_new(int32_t n) {
    //     return std::make_unique<ServiceNet>(n);
    // }

    void connect_to_tcp_server(ServiceWrapper* srv, ServiceWrapper* srvNet, rust::String name, rust::string addr, TcpCallbacks handler) {
        //handler.on_listen(*srv, "abc");
    }

} // namespace commlib

#endif // __NET_BINDINGS_H__
