/* 内核的初始化 */

#include "Utility.h"
#include "Video.h"
#include "IOPort.h"
#include "Chip8259A.h"
#include "Machine.h"
#include "Assembly.h"
#include "Kernel.h"

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

extern "C" void main0(void);

extern "C" void kernelBridge() {  // called by sector2.asm
	initBss();
	callCtors();
	main0();
	callDtors();
}

