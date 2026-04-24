#include "Process.h"

extern "C" {
	void Process_sleep_kernel(Process*, unsigned long, int);
}

void Process::Sleep(unsigned long chan, int pri) {
	Process_sleep_kernel(this, chan, pri);
}
