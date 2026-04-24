#ifndef MACHINE_H
#define	MACHINE_H

#include "PageDirectory.h"
#include "sys/types.h"

/*
 * Machine类用于封装对底层硬件、保护模式下数据结构的抽象。
 * 包括对8254时钟芯片、8259A中断控制芯片的初始化，以及对
 * 保护模式下GDT, IDT等数据结构的操作。
 * 
 * Machine类使用Singleton模式实现，在系统内核整个生命周期
 * 中只有一个实例对象。
 */
class Machine
{
	/* static const member */
public:
	/* 内核代码段、内核数据段，用户代码段、用户数据段，TSS段的选择子 */
	static const unsigned int KERNEL_CODE_SEGMENT_SELECTOR = 0x08;
	static const unsigned int KERNEL_DATA_SEGMENT_SELECTOR = 0x10;
	static const unsigned int USER_CODE_SEGMENT_SELECTOR = (0x18 | 0x3);
	static const unsigned int USER_DATA_SEGMENT_SELECTOR = (0x20 | 0x3);		
	static const unsigned int TASK_STATE_SEGMENT_SELECTOR = 0x28;
	static const unsigned int TASK_STATE_SEGMENT_IDX = 0x5;	/* TSS段描述符在GDT中的位置 */

	/* 页目录、核心态页表、用户态页表在物理内存中的起始地址 */
	static const unsigned long PAGE_DIRECTORY_BASE_ADDRESS = 0x200000;
	static const unsigned long KERNEL_PAGE_TABLE_BASE_ADDRESS = 0x201000;

	static const unsigned long VESA_PAGE_TABLE_BASE_ADDR = 32 * 1024 * 1024;  // place it over 32M

	static const unsigned long USER_PAGE_TABLE_BASE_ADDRESS = 0x202000;
	static const unsigned long USER_PAGE_TABLE_CNT = 2;
	
	/* 内核空间大小 4M 0xC0000000 - 0xC0400000 1 PageTable */
	static const unsigned int KERNEL_SPACE_SIZE = 0x400000;
	static const unsigned long KERNEL_SPACE_START_ADDRESS	= 0xC0000000;
	
public:
	static Machine& Instance();			/* 返回单态类的instance */
	void LoadTaskRegister();

	/**
	 * VESA Support
	 *   added by 2051565 GTY
	 */
	void InitVESAMemoryMap(uintptr_t videoMemAddr, uintptr_t virtualMemAddr, size_t videoMemSize);

	void InitUserPageTable();
private:
	static Machine instance;	/* Machine单体类实例 */
};

#endif
