#ifndef __SERVICE_NET_H__
#define __SERVICE_NET_H__

#include <functional>
#include "rust/cxx.h"
#include "packet.h"

struct ServiceWrapper;

namespace commlib
{
    class Service
    {
    public:
        int64_t id;
    };

    ////
    using RunFunc = std::function<void>();

    ////
    class ServiceNet : public Service
    {
    public:
        ServiceNet( int packetMemLimit )
        {
            
        }

        void OnConnection(struct ServiceWrapper* srv) {}

        void Init(struct ServiceWrapper* srv) {}

        void run_in_service(RunFunc exec) {
            printf("hello");
        }

    public:
        int64_t num = 0;
       
    };

} // namespace commlib

#endif // __SERVICE_NET_H__
