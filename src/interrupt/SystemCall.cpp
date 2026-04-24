#include "SystemCall.h"
#include "CharDevice.h"
#include "User.h"
#include "Kernel.h"

extern "C" {
unsigned int get_time();
void time_set(unsigned int value);
unsigned int time_tout();
void time_set_tout(unsigned int value);
unsigned long time_tout_address();
}

SystemCall::SystemCall()
{
	//nothing to do here
}

SystemCall::~SystemCall()
{
	//nothing to do here
}

void SystemCall::Trap1(int (*func)())
{
	User& u = Kernel::Instance().GetUser();

	User_get_intflg() = 1;
	func();
	User_get_intflg() = 0;
}

void SystemCall::Trap1ByNumber(unsigned int number)
{
	switch ( number )
	{
	case 7: Trap1(Sys_Wait); return;
	case 11: Trap1(Sys_Exec); return;
	case 17: Trap1(Sys_SBreak); return;
	case 31: Trap1(Sys_Stty); return;
	case 32: Trap1(Sys_Gtty); return;
	case 34: Trap1(Sys_Nice); return;
	case 35: Trap1(Sys_Sslep); return;
	case 37: Trap1(Sys_Kill); return;
	case 48: Trap1(Sys_Ssig); return;
	default:
		User_get_error() = User::ENOSYS;
		return;
	}
}

extern "C" void cpp_system_call_trap1(unsigned int number)
{
	SystemCall::Trap1ByNumber(number);
}

extern "C" void ProcessManager_wait();
/*	7 = wait	count = 0	*/
int SystemCall::Sys_Wait()
{
	ProcessManager_wait();
	return 0;	/* GCC likes it ! */
}

extern "C" void ProcessManager_exec();
/*	11 = exec	count = 2	*/
int SystemCall::Sys_Exec()
{
	ProcessManager_exec();
	return 0;	/* GCC likes it ! */
}

/*	17 = sbreak	count = 1	*/
int SystemCall::Sys_SBreak()
{
	User& u = Kernel::Instance().GetUser();
	User_get_procp()->SBreak();

	return 0;	/* GCC likes it ! */
}

/*	31 = stty	count = 1	*/
int SystemCall::Sys_Stty()
{
    // TODO
	/*Inode* pInode;
	User& u = Kernel::Instance().GetUser();
	int fd = User_get_arg()[0];
	TTy* pTTy = (TTy *)User_get_arg()[1];

	if ( (pInode = OpenFiles_get_inode(fd)) == NULL )
	{
		return 0;
	}
	if ( (pInode->i_mode & Inode::IFMT) != Inode::IFCHR )
	{
		User_get_error() = User::ENOTTY;
		return 0;
	}
	short dev = Inode_get_dev(pInode);
        g_ConsoleDevice.SgTTy(dev, pTTy);*/

	return 0;	/* GCC likes it ! */
}

/*	32 = gtty	count = 1	*/
int SystemCall::Sys_Gtty()
{
    // TODO
	/*Inode* pInode;
	User& u = Kernel::Instance().GetUser();
	int fd = User_get_arg()[0];
	TTy* pTTy = (TTy *)User_get_arg()[1];

	if ( (pInode = OpenFiles_get_inode(fd)) == NULL )
	{
		return 0;
	}
	if ( (pInode->i_mode & Inode::IFMT) != Inode::IFCHR )
	{
		User_get_error() = User::ENOTTY;
		return 0;
	}
	short dev = Inode_get_dev(pInode);
        g_ConsoleDevice.SgTTy(dev, pTTy);*/

	return 0;	/* GCC likes it ! */
}

/*	34 = nice	count = 0	*/
int SystemCall::Sys_Nice()
{
	User& u = Kernel::Instance().GetUser();
	User_get_procp()->Nice();

	return 0;	/* GCC likes it ! */
}

/*	35 = sleep	count = 0	*/
int SystemCall::Sys_Sslep()
{
	User& u = Kernel::Instance().GetUser();

	X86Assembly::CLI();

	unsigned int wakeTime = get_time() + User_get_arg()[0];	/* sleep(second) */

	/*
	 * 对   if ( Time::tout <= Time::time || Time::tout > wakeTime )  中判断条件的解释：
	 * 1、系统先前设置的所有闹钟均已到期。  其后，第一个设置闹钟的进程看到的是条件 tout <= time成立，将自己的waketime写入tout变量。
	 * 2、系统中，存在闹钟未到期的进程。如果有进程设置闹钟，看到的是条件tout > time，进程比对tout变量和自己的waketime，令tout变量的值是所有进程waketime的最小值。
	 *
	 * 原先的注释：
	 * 此处不可以'wakeTime >= Time::time', 否则极端情况下前一次sleep(sec)刚结束，
	 * 紧接着第二次sleep(0)，会使wakeTime == Time::time == Time::tout，
	 * 而如果此时发生时钟中断恰为一秒末尾，Time::Clock()中Time::time++，
	 * 会导致Time::tout比Time::time小1，永远无法满足Time::time == Time::tout
	 * 的唤醒条件，调用sleep(0)的进程永远睡眠。         The end.
	 *
	 * 原先的注释不对。如果while循环的判断条件是'wakeTime >= Time::time'，执行sleep(0)的进程将把waketime和tout设为上个整数秒。整数秒时钟中断处理程序会time++，之后
	 * 1、如果不再有进程设置新闹钟，系统的闹钟服务就瘫痪了。这是因为， time==tout的条件永远无法满足，时钟中断处理程序不再会唤醒任何因设置了闹钟而入睡的进程。
	 * 2、如果有进程设置新闹钟newWaketime，执行sleep(0)操作的进程以及所有waketime<=newWaketime的进程的唤醒时刻将推迟到newWaketime。
	 *
	 * 现在的闹钟服务正确，执行sleep(0)的进程不会入睡更不会使tout值出现错误。
	 */
	while( wakeTime > get_time() )
	{
		unsigned int now = get_time();
		unsigned int tout = time_tout();
		if ( tout <= now || tout > wakeTime )
		{
			time_set_tout(wakeTime);
		}
		User_get_procp()->Sleep(time_tout_address(), ProcessManager::PSLEP);
	}

	X86Assembly::STI();

	return 0;	/* GCC likes it ! */
}

extern "C" void ProcessManager_kill();
/*	37 = kill	count = 1	*/
int SystemCall::Sys_Kill()
{
	ProcessManager_kill();
	return 0;	/* GCC likes it ! */
}

/*	48 = ssig	count = 2	*/
int SystemCall::Sys_Ssig()
{
	User& u = Kernel::Instance().GetUser();
	User_get_procp()->Ssig();

	return 0;	/* GCC likes it ! */
}
