#ifndef __SIGNAL_BINDINGS_H__
#define __SIGNAL_BINDINGS_H__

#include "rust/cxx.h"

using SignalCallback = void(*)(int32_t);

namespace commlib
{
    void init_signal_handlers(SignalCallback cb_ctrl_c, SignalCallback cb_usr1, SignalCallback cb_usr2);
    //void init_signal_handlers();

    void new_abc();

} // namespace commlib

#endif // __SIGNAL_BINDINGS_H__
