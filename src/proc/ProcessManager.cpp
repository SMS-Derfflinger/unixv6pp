#include "ProcessManager.h"
#include "User.h"
#include "Kernel.h"
#include "Video.h"
#include "Utility.h"

unsigned int ProcessManager::m_NextUniquePid = 0;

ProcessManager::ProcessManager()
{
	CurPri = 0;
	RunRun = 0;
	RunIn = 0;
	RunOut = 0;
	ExeCnt = 0;
	SwtchNum = 0;
}

ProcessManager::~ProcessManager()
{
}

void ProcessManager::Initialize()
{
	//nothing to do here
}

extern "C" void Userspace_init();

extern "C" void* Userspace_before_fork();
extern "C" void Userspace_after_fork(void*);

/*
 * Sched() 和 Wait() 已迁移至 Rust (manager.rs)。
 * 进程表现在由 Rust 的 ProcessManager.procs 管理。
 */

extern "C" void ProcessManager_sched();
void ProcessManager::Sched()
{
	ProcessManager_sched();
}

extern "C" void ProcessManager_wait();
void ProcessManager::Wait()
{
	ProcessManager_wait();
}

extern "C" void ProcessManager_wakeup_all_chan(unsigned long chan);
void ProcessManager::WakeUpAll(unsigned long chan)
{
	ProcessManager_wakeup_all_chan(chan);
}

extern "C" int ProcessManager_swtch();
int ProcessManager::Swtch()
{
	return ProcessManager_swtch();
}

extern "C" void ProcessManager_fork();
void ProcessManager::Fork()
{
	ProcessManager_fork();
}

extern "C" void ProcessManager_exec();
void ProcessManager::Exec()
{
	ProcessManager_exec();
}

extern "C" void ProcessManager_kill();
void ProcessManager::Kill()
{
	ProcessManager_kill();
}

extern "C" int ProcessManager_new_proc();
int ProcessManager::NewProc()
{
	return ProcessManager_new_proc();
}

extern "C" void ProcessManager_setup_proc_zero();
void ProcessManager::SetupProcessZero()
{
	ProcessManager_setup_proc_zero();
}

extern "C" void ProcessManager_xswap(Process* p, bool free_mem, int size);
void ProcessManager::XSwap(Process* pProcess, bool bFreeMemory, int size)
{
	ProcessManager_xswap(pProcess, bFreeMemory, size);
}

extern "C" Process* ProcessManager_select();
Process* ProcessManager::Select()
{
	return ProcessManager_select();
}

void ProcessManager::Signal(TTy* pTTy, int signal)
{
	/* 已迁移至 Rust，此处暂不实现 */
}

unsigned int ProcessManager::NextUniquePid()
{
	return m_NextUniquePid++;
}

extern "C" void runtime();
extern "C" void ExecShell();
