/* 内核的初始化 */

#include "Utility.h"
#include "Video.h"
#include "IOPort.h"
#include "Chip8253.h"
#include "Chip8259A.h"
#include "Machine.h"
#include "Assembly.h"
#include "Kernel.h"
#include "OpenFileManager.h"
#include "CMOSTime.h"

#include "libyrosstd/sys/types.h"

#ifdef __cplusplus
extern "C" {
#endif

void clear_screan();

#ifdef __cplusplus
}
#endif

extern "C" void MasterIRQ7()
{
	SaveContext();

	Diagnose::Write("IRQ7 from Master 8259A!\n");

	//需要在中断处理程序末尾先8259A发送EOI命令
	//实验发现：有没有下面IOPort::OutByte(0x27, 0x20);这句运行效果都一样，本来以为
	//发送EOI命令之后会有后续的IRQ7中断进入， 但试下来结果是IRQ7只会产生一次。
	IOPort::OutByte(Chip8259A::MASTER_IO_PORT_1, Chip8259A::EOI);

	RestoreContext();

	Leave();

	InterruptReturn();
}


static void callCtors()
{
	extern void (*__CTOR_LIST__)();
	extern void (* __CTOR_END__)();


	void (**constructor)() = &__CTOR_LIST__;


	//constructor++;
		/*  (可以先看一下链接脚本：Link.ld)
		Link script中修改过后，这里的total已经不是constructor的个数了，
		_CTOR_LIST__的第一个单元开始就是global/static对象的constructor，
		所以不用 constructor++;
		*/

	while(constructor != &__CTOR_END__) //total不是constructor的数量，而是用于检测是否到了_CTOR_LIST__的末尾
	{
		(*constructor)();
		constructor++;
	}
}

static void initBss() {  // https://github.com/FlowerBlackG/YurongOS/blob/master/src/misc/main.cpp
	extern unsigned int __BSS_START__;
    extern unsigned int __BSS_END__;


    unsigned int bssStart = (unsigned int) &__BSS_START__;
    unsigned int bssEnd = (unsigned int) &__BSS_END__;

    for (unsigned int pos = bssStart; pos < bssEnd; pos++) {
        * ((char*) pos) = 0;
    }
}


static void callDtors()
{
	extern void (* __DTOR_LIST__)();
	extern void (* __DTOR_END__)();

	void (**deconstructor)() = &__DTOR_LIST__;

	while(deconstructor != &__DTOR_END__)
	{
		(*deconstructor)();
		++deconstructor;
	}
}

/* 应用程序从main返回，进程就终止了，这全是runtime()的功劳。没有它，就只能用exit终止进程了。xV6没这个功能^-^ */
extern "C" void runtime()
{
	/*
	1. 销毁runtime的stack Frame
	2. esp中指向用户栈中argc位置，而ebp尚未正确初始化
	3. eax中存放可执行程序EntryPoint
	4~6. exit(0)结束进程
	*/
	__asm("	leave;	\
			movl %%esp, %%ebp;	\
			call *%%eax;		\
			movl $1, %%eax;	\
			movl $0, %%ebx;	\
			int $0x80"::);
}

/*
  * 1#进程在执行完MoveToUserStack()从ring0退出到ring3优先级后，会调用ExecShell()，此函数通过"int $0x80"
  * (EAX=execv系统调用号)加载“/Shell.exe”程序，其功能相当于在用户程序中执行系统调用execv(char* pathname, char* argv[])。
  */
extern "C" void ExecShell()
{
	int argc = 0;
	char* argv = NULL;
	const char* pathname = "/Shell.exe";
	__asm ("int $0x80"::"a"(11/* execv */),"b"(pathname),"c"(argc),"d"(argv));
	return;
}

extern "C" void InitProcessEntry()
{
	Machine::Instance().InitUserPageTable();
	FlushPageDirectory();

	clear_screan();

	/* 1#进程回用户态，执行exec("shell.exe")系统调用 */
	MoveToUserStack();
	__asm ("call *%%eax" :: "a"((unsigned long)ExecShell - 0xC0000000));

	Utility::Panic("InitProcessEntry returned");
}

extern "C" void main0(void);

extern "C" void kernelBridge() {  // called by sector2.asm
	initBss();
	callCtors();
	main0();
	callDtors();
}

