#include "signal_bindings.h"
#include <signal.h>  // for signal, SIGABRT, SIGINT, SIGPIPE

namespace commlib
{
	void init_signal_handlers(SignalCallback cb_ctrl_c, SignalCallback cb_usr1, SignalCallback cb_usr2)
	{
		signal(SIGINT, cb_ctrl_c);
		signal(SIGTERM, cb_ctrl_c);
		signal(SIGABRT, cb_ctrl_c);

#ifdef _WIN32
		signal(SIGBREAK, cb_ctrl_c);
#else
		signal(SIGQUIT, cb_ctrl_c);

		signal(SIGPIPE, SIG_IGN); // ignore signal

		signal(SIGUSR1, cb_usr1); // 关服
		signal(SIGUSR2, cb_usr2); // 热更新配置
#endif
	}

	void new_abc()
	{
		printf("test new_abc");
	}
}
