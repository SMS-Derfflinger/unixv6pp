#include "ProcessManager.h"
#include "Machine.h"
#include "User.h"
#include "Kernel.h"
#include "Video.h"
#include "Utility.h"
#include "PEParser.h"
#include "Regs.h"
#include "New.h"
#include "MemoryDescriptor.h"

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

void ProcessManager::Sched()
{
	Process* pSelected;
	User& u = Kernel::Instance().GetUser();
	int seconds;
	unsigned int size;
	unsigned long desAddress;

	/*
	 * 选择在交换区驻留时间最长，处于就绪状态的进程换入
	 */
	goto loop;

sloop:
	this->RunIn++;
	User_get_procp()->Sleep((unsigned long)&RunIn, ProcessManager::PSWP);

loop:
	X86Assembly::CLI();
	seconds = -1;
	for ( int i = 0; i < ProcessManager::NPROC; i++ )
	{
		if ( this->process[i].p_stat == Process::SRUN && (this->process[i].p_flag & Process::SLOAD) == 0 && this->process[i].p_time > seconds )
		{
			pSelected = &(this->process[i]);
			seconds = pSelected->p_time;
		}
	}

	/* 如果没有符合条件的进程，0#进程睡眠等待有需要换入的进程 */
	if ( -1 == seconds )
	{
		this->RunOut++;
		User_get_procp()->Sleep((unsigned long)&RunOut, ProcessManager::PSWP);
		goto loop;
	}

	/* 如果有进程满足条件，需要换入，则检查是否有足够内存 */
	X86Assembly::STI();
	/* 计算进程换入需要的内存大小 */
	size = pSelected->p_size;
	/*
	 * 如果存在共享正文段，但是没有进程图像在内存中，引用该正文段的进程，
	 * 即共享正文段不再内存中，换入时需要读入正文段在交换区中的副本
	 */
	if ( pSelected->p_textp != NULL && 0 == pSelected->p_textp->x_ccount )
	{
		size += pSelected->p_textp->x_size;
	}
	/* 如果内存分配成功，则进行实际换入操作 */
	desAddress = Kernel::Instance().GetUserPageManager().AllocMemory(size);
	if ( NULL != desAddress )
	{
		goto found2;
	}

	/*
	 * 分配内存失败情况下，换出内存中进程，腾出空间。
	 * 换出原则：从易到难；依次将低优先权睡眠状态(SWAIT)-->
	 * 暂停状态(SSTOP)-->高优先权睡眠状态(SSLEEP)-->就绪状态(SRUN)进程换出。
	 */
	X86Assembly::CLI();
	for ( int i = 0; i < ProcessManager::NPROC; i++ )
	{

		bool pFlagIsSLOAD = (this->process[i].p_flag & (int(Process::SSYS) | int(Process::SLOCK) | int(Process::SLOAD))) == int(Process::SLOAD);

		bool statIsSWAITOrSSTOP = (this->process[i].p_stat == Process::SWAIT || this->process[i].p_stat == Process::SSTOP);

		if (pFlagIsSLOAD && statIsSWAITOrSSTOP)
		{
			goto found1;
		}
	}

	/*
	 * 在换出高优先权睡眠状态(SSLEEP)、就绪状态(SRUN)进程而腾出内存之前，
	 * 检查待换入进程在交换区驻留时间是否已达到3秒，低于则不予换入
	 */
	if ( seconds < 3 )
	{
		goto sloop;
	}

	seconds = -1;
	for ( int i = 0; i < ProcessManager::NPROC; i++ )
	{

		bool pFlagIsSLOAD = (this->process[i].p_flag & (int(Process::SSYS) | int(Process::SLOCK) | int(Process::SLOAD))) == int(Process::SLOAD);
		bool pStatIsSWAITOrSSTOP = this->process[i].p_stat == Process::SWAIT || this->process[i].p_stat == Process::SSTOP;

		if ( pFlagIsSLOAD && pStatIsSWAITOrSSTOP && pSelected->p_time > seconds ) {
			pSelected = &(this->process[i]);
			seconds = pSelected->p_time;
		}
	}

	/* 如果要换出SSLEEP、SRUN状态进程，先检查该进程驻留内存时间是否超过2秒，否则不予换出 */
	if ( seconds < 2 )
	{
		goto sloop;
	}

	/* 换出pSelected指向的被选中进程 */
found1:
	X86Assembly::STI();
	pSelected->p_flag &= ~Process::SLOAD;
	this->XSwap(pSelected, true, 0);
	/* 腾出内存空间后再次尝试换入进程 */
	goto loop;

	/* 已经分配好足够的内存，进行实际的换入操作 */
found2:
	BufferManager& bufMgr = Kernel::Instance().GetBufferManager();
	/*
	* 如果存在共享正文段，但是没有进程图像在内存中，引用该正文段的进程，
	* 即共享正文段不再内存中，换入时需要读入正文段在交换区中的副本
	*/
	if ( pSelected->p_textp != NULL )
	{
		Text* pText = pSelected->p_textp;
		if ( pText->x_ccount == 0 )
		{
			/* 因为共享正文段，和进程ppda、数据段、堆栈段在交换区中是分开存放的，所以先换入共享正文段 */
			if ( bufMgr.Swap(pText->x_daddr, desAddress, pText->x_size, Buf::B_READ) == false )
			{
				goto err;
			}
			/* 共享正文段在内存中的起始地址 */
			pText->x_caddr = desAddress;
			desAddress += pText->x_size;
		}
		pText->x_ccount++;
	}
	/* 换入剩余部分图像：ppda、数据段、堆栈段 */
	if ( bufMgr.Swap(pSelected->p_addr /* blkno */, desAddress, pSelected->p_size, Buf::B_READ) == false )
	{
		goto err;
	}
	Kernel::Instance().GetSwapperManager().FreeSwap(pSelected->p_size, pSelected->p_addr /* blkno */);
	pSelected->p_addr = desAddress;
	pSelected->p_flag |= Process::SLOAD;
	pSelected->p_time = 0;
	goto loop;

err:
	Utility::Panic("Swap Error");
}

void ProcessManager::Wait()
{
	int i;
	bool hasChild = false;
	User& u = Kernel::Instance().GetUser();
	SwapperManager& swapperMgr = Kernel::Instance().GetSwapperManager();
	BufferManager& bufMgr = Kernel::Instance().GetBufferManager();

	Diagnose::Write("Process %d finding dead son. They are ",User_get_procp()->p_pid);
	while(true)
	{
		for ( i = 0; i < NPROC; i++ )
		{
			if ( User_get_procp()->p_pid == process[i].p_ppid )
			{
				Diagnose::Write("Process %d (Status:%d)  ",process[i].p_pid,process[i].p_stat);
				hasChild = true;
				/* 睡眠等待直至子进程结束 */
				if( Process::SZOMB == process[i].p_stat )
				{
					/* wait()系统调用返回子进程的pid */
					User_get_ar0()[User::EAX] = process[i].p_pid;

					process[i].p_stat = Process::SNULL;
					process[i].p_pid = 0;
					process[i].p_ppid = -1;
					process[i].p_sig = 0;
					process[i].p_flag = 0;

					/* 读入swapper中子进程u结构副本 */
					Buf* pBuf = bufMgr.Bread(DeviceManager::ROOTDEV, process[i].p_addr);
					swapperMgr.FreeSwap(BufferManager::BUFFER_SIZE, process[i].p_addr);
					User* pUser = (User *)pBuf->b_addr;

					/* 把子进程的时间加到父进程上 */
                                        // greatbridf: don't consider this for now.
                                        // maybe add them back later...
					// User_get_cstime() += pUser->u_cstime +	pUser->u_stime;
					// User_get_cutime() += pUser->u_cutime + pUser->u_utime;

					int* pInt = (int *)User_get_arg()[0];
					/* 获取子进程exit(int status)的返回值 */
                                        // greatbridf: this is the same
					// *pInt = pUser->u_arg[0];

					/* 如果此处没有Brelse()系统会发生什么-_- */
					bufMgr.Brelse(pBuf);
					Diagnose::Write("end wait\n");
					return;
				}
			}
		}
		if (true == hasChild)
		{
			/* 睡眠等待直至子进程结束 */
			Diagnose::Write("wait until child process Exit! ");
			User_get_procp()->Sleep((unsigned long)User_get_procp(), ProcessManager::PWAIT);
			Diagnose::Write("end sleep\n");
			continue;	/* 回到外层while(true)循环 */
		}
		else
		{
			/* 不存在需要等待结束的子进程，设置出错码，wait()返回 */
			User_get_error() = User::ECHILD;
			break;	/* Get out of while loop */
		}
	}
}

extern "C" void runtime();
extern "C" void ExecShell();

extern "C" void _wakeup_all(unsigned long chan) {
	Kernel::Instance().GetProcessManager().WakeUpAll(chan);
}
