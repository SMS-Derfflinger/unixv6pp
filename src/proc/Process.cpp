#include "Process.h"
#include "Kernel.h"

extern "C" {
	void Process_send_signal(Process*);
	void Process_process_signal(Process*, struct pt_context*);
	void Process_set_nice(Process*);
	bool Process_should_process(Process*);
	void Process_raise(Process*, int);
	void Process_set_pri(Process*);
	void Process_exit(Process*);
	void Process_sstack(Process*);
	void Process_sbrk(Process*);
	void Process_sleep_kernel(Process*, unsigned long, int);
}

extern "C" int alloc_swap(unsigned long len) {
        return Kernel::Instance().GetSwapperManager().AllocSwap(len);
}

extern "C" void compat_swap_free(unsigned int blkno) {
	Kernel::Instance().GetSwapperManager().FreeSwap(0, blkno);
}

Process::Process() {}
Process::~Process() {}

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

void Process::PSig(struct pt_context* pContext) {
	Process_process_signal(this, pContext);
}

void Process::Nice() {
	Process_set_nice(this);
}

void Process::Ssig() {
	Process_send_signal(this);
}

void Process::SetPri() {
	Process_set_pri(this);
}

void Process::Exit() {
	Process_exit(this);
}

void Process::SStack() {
	Process_sstack(this);
}

void Process::SBreak() {
	Process_sbrk(this);
}

void Process::Sleep(unsigned long chan, int pri) {
	Process_sleep_kernel(this, chan, pri);
}
