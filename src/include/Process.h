#ifndef PROCESS_H
#define PROCESS_H

#include "Regs.h"

/*
 * Process类与UNIX V6中进程控制块proc结构对应，这里只改变
 * 类名，不修改成员结构名字，以及对UNIX V6的proc结构中成员
 * 使用的数据类型进行适当更改，以符合现代程序的代码风格。
 */
class Process
{
public:
	enum ProcessState	/* 进程状态 */
	{
		SNULL	= 0,	/* 未初始化空状态 */
		SSLEEP	= 1,	/* 高优先权睡眠 */
		SWAIT	= 2,	/* 低优先权睡眠 */
		SRUN	= 3,	/* 运行、就绪状态 */
		SIDL	= 4,	/* 进程创建时的中间状态 */
		SZOMB	= 5,	/* 进程终止时的中间状态 */
		SSTOP	= 6		/* 进程正被跟踪 */
	};

	enum ProcessFlag	/* 进程标志位 */
	{
		SLOAD	= 0x1,	/* 进程图像在内存中 */
		SSYS	= 0x2,	/* 系统进程图像，不允许被换出 */
		SLOCK	= 0x4,	/* 含有该标志的进程图像暂不允许换出 */
		SSWAP	= 0x8,	/* 该进程被创建时图像就在交换区上 */
		STRC	= 0x10,	/* 父子进程跟踪标志，UNIX V6++未有效使用到 */
		STWED	= 0x20	/* 父子进程跟踪标志，UNIX V6++未有效使用到 */
	};
public:
	Process();
	~Process();

	void Sleep(unsigned long chan, int pri);	/* 使当前进程转入睡眠状态 */

	void SStack();                              /* 堆栈溢出时，自动扩展堆栈 */

	void PSignal(int signal);					/* 向当前进程发送信号。signal是要发送的信号数 */

	void PSig(struct pt_context* pContext);		/* 对当前进程接收到的信号进行处理 */

	void Ssig();								/* 设置用户自定信号处理方式的系统调用处理函数 */

	int IsSig();								/* 判断当前进程是否接收到信号。
												 * 
												 * 进程在每次中断、异常和系统调用末尾退出核心态时，以及低优先权
												 * 睡眠首尾处总会检查p_sig。p_sig中接收到信号本身对进程并无影响，
												 * 仅在响应信号时进程会对改变原定执行流程。
												 * 
												 * 返回值：信号数，无信号则返回0。
												 */
};

#endif
