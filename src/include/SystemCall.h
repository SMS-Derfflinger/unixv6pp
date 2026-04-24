#ifndef SYSTEM_CALL_H
#define SYSTEM_CALL_H

/*
 * UNIX V6中使用编译后trap指令码的低6bit作为index查找入口表，这依靠
 * trap指令能够针对不同系统调用产生不同指令码。而X86平台上的int指令
 * 无法做到产生不同的指令码，因而通过eax寄存器传入系统调用号作为index。
 * 
 * eax中存放系统调用号，作为查找入口表中函数的index。
 * ebx开始存放用户态程序提供的系统调用第一个参数，ecx第二参数，以此类推
 * ebp存放最后的参数；最多可以有6个参数。其实UNIX V6的系统调用参数最多只有4个。
 *　
 * 随后会将传入参数转存到User_get_arg()[5]中。
 */
class SystemCall
{
public:
	/*系统调用处理程序入口表的大小*/
	static const unsigned int SYSTEM_CALL_NUM = 64;

public:
	SystemCall();
	~SystemCall();

public:
	/* 对应UNIX V6中的trap1( int (*f)() )函数@line 2841
	 * 此函数由trap(dev,...)函数调用，trap(dev,...)函数
	 * 提供从入口表中获取的函数指针，作为参数传递给trap1( int (*f)());
	 */
	static void Trap1(int (*func)());

	static void Trap1ByNumber(unsigned int number);

private:
	/* 下面的函数对应系统调用入口表中的处理程序入口地址,
	 * 他们负责系统调用在核心态下进行的具体处理逻辑。
	 *
	 * 这里函数统一声明为int func(void);而系统调用的返回值
	 * 并不是通过int返回，只是为了和int (*call)()类型匹配。
	 *
	 * UNIX V6中返回值放在User_get_ar0()[R0]中，也就是通过r0寄存器
	 * 返回，而这里考虑使用EAX寄存器返回系统调用结果给用户态
	 * 程序。
	 */

	/*	17 = sbreak	count = 1	*/
	static int Sys_SBreak();

	/*	31 = stty	count = 1	*/
	static int Sys_Stty();
	
	/*	32 = gtty	count = 1	*/
	static int Sys_Gtty();
	
	/*	33 = nosys	count = 0	*/
	
	/*	34 = nice	count = 0	*/
	static int Sys_Nice();
	
	/*	35 = sleep	count = 0	*/
	static int Sys_Sslep();		/* Don't Confused with sleep(chan, pri) */

	/*	48 = sig	count = 2	*/
	static int Sys_Ssig();
	
	/*	49 ~ 63 = nosys	count = 0	*/	


};

#endif
