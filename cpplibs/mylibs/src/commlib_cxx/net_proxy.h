#ifndef __NET_PROXY_H__
#define __NET_PROXY_H__

#include "rust/cxx.h"
#include "packet.h"
#include "service_net.h"

namespace commlib
{
    ////
    template<NetPacket::PacketType type>
    class NetProxy
    {
    public:
        NetProxy(ServiceNet* srvNet)
        {

        }

        ~NetProxy() {}

        void OnNetData(int hd, NetPacket* pkt)
        {
        }

        void SendRaw(int hd, int cmd, const char* data, int len)
        {

        }
    };

} // namespace commlib

#endif // __NET_PROXY_H__
