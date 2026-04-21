#include "Process.h"
#include "Kernel.h"

extern "C" {
	void Process_send_signal(Process*);
	void Process_process_signal(Process*, struct pt_context*);
	void Process_set_nice(Process*);
	bool Process_should_process(Process*);
	void Process_raise(Process*, int);
}

extern "C" void _sleep(unsigned long chan, int pri) {
        User_get_procp()->Sleep(chan, pri);
}

extern "C" int alloc_swap(unsigned long len) {
        return Kernel::Instance().GetSwapperManager().AllocSwap(len);
}

extern "C" void compat_set_run(Process* proc) {
	proc->SetRun();
}

void Process::PSignal( int signal )
{
	Process_raise(this, signal);
}

int Process::IsSig()
{
	return Process_should_process(this);
}

/*
extern "C" void runtime();
extern "C" void SignalHandler();
*/

extern "C" void user_exit() {
	User_get_procp()->Exit();
}

void Process::PSig(struct pt_context* pContext) {
	Process_process_signal(this, pContext);
}

void Process::Nice() {
	Process_set_nice(this);
}

void Process::Ssig() {
	Process_send_signal(this);
}
