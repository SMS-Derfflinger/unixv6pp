#ifndef PROCESS_MANAGER_H
#define PROCESS_MANAGER_H

#include "Process.h"
#include "Assembly.h"

class ProcessManager
{
	/* static consts */
public:
	/*
	 * 进程进入睡眠状态时，内核根据其睡眠原因设置其醒来后的优先数；
	 * 优先数小于零为高优先权睡眠，优先数大于零为低优先权睡眠。
	 */
	static const int PSWP = -100;
	static const int PINOD = -90;
	static const int PRIBIO = -50;
	static const int EXPRI = -1;
	static const int PPIPE = 1;
	static const int TTIPRI = 10;
	static const int TTOPRI = 20;
	static const int PWAIT = 40;
	static const int PSLEP = 90;
	static const int PUSER = 100;
};

#endif

