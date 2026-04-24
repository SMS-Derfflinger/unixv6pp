/* 内核的初始化 */

#include "Utility.h"
#include "Video.h"
#include "IOPort.h"
#include "Chip8259A.h"
#include "Machine.h"
#include "Assembly.h"
#include "Kernel.h"

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

