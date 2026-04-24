#include "Process.h"

extern "C" {
	void Process_process_signal(Process*, struct pt_context*);
	bool Process_should_process(Process*);
	void Process_raise(Process*, int);
	void Process_sstack(Process*);
	void Process_sleep_kernel(Process*, unsigned long, int);
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

void Process::SStack() {
	Process_sstack(this);
}

void Process::Sleep(unsigned long chan, int pri) {
	Process_sleep_kernel(this, chan, pri);
}
