#ifndef __PACKET_H__
#define __PACKET_H__

#include "rust/cxx.h"

namespace commlib
{
    class NetPacket
    {
    public:
        enum PacketType
        {
            SERVER,
            CLIENT,
            ROBOT,

            CLIENT_WEB_SOCKET,
            ROBOT_WEB_SOCKET,
        };

        struct ClientHead
        {
            uint8_t no_{};
        };

    public:
        static NetPacket* TakePacket(size_t size);
        static void ReleasePakcet(NetPacket*);

    public:
    };
    

} // namespace commlib

#endif // __NET_PROXY_H__
