#ifndef __SIGNAL_H__
#define __SIGNAL_H__

#include "rust/cxx.h"

using SignalCallback = void(*)(int32_t);

namespace commlib_cxx
{
    void init_signal_handlers(SignalCallback cb_ctrl_c, SignalCallback cb_usr1, SignalCallback cb_usr2);

    void new_abc();

} // namespace commlib

#endif // __SIGNAL_H__
