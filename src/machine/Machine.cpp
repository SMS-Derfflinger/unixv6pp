#include "Machine.h"
#include "Assembly.h"
#include "Exception.h"
#include "TimeInterrupt.h"
#include "DiskInterrupt.h"
#include "KeyboardInterrupt.h"
#include "SystemCall.h"

#include "PageManager.h"


Machine Machine::instance;	/*单态类实例的定义*/

TaskStateSegment g_TaskStateSegment;

Machine& Machine::Instance()
{
	return instance;
}

void Machine::LoadIDT()
{
	IDTR idtr;
	IDT::FormIDTR(idtr);
	X86Assembly::LIDT((unsigned short*)(&idtr));
}

void Machine::LoadGDT()
{
	GDTR gdtr;
	GDT::FormGDTR(gdtr);
	X86Assembly::LGDT((unsigned short*)(&gdtr));
}

void Machine::LoadTaskRegister()
{
	X86Assembly::LTR(Machine::TASK_STATE_SEGMENT_SELECTOR);
}

extern "C" void MasterIRQ7();

void Machine::InitIDT()
{
	GDT::Initialize();
	/*
	 * 1. 将IDT中0 - 255个表项全部填入默认中断/异常处理函数入口，确
	 *    保任意一种中断/异常发生时都会被处理，避免内核崩溃。
	 * 2. 对INT 0 - 31号异常，使用非默认的中断/异常处理程序覆盖先前
	 *    默认处理函数入口。
	 * 3. 设置时钟中断、键盘中断、磁盘中断等对应的中断入口。
	 * 4. INT 0 - 31号异常中使用默认处理函数的，一般情况下不可能发生，
	 *    对于这些异常的处理流程不进行现场保存和恢复，仅输出错误信息，
	 *    进入死循环，等待人工干预。
	 */
	for ( int i = 0; i <= 255; i++ )
	{
		 if( i < 32 )
			 IDT::SetTrapGate(i, (unsigned long)IDT::DefaultExceptionHandler); 
		 else
			 IDT::SetInterruptGate(i, (unsigned long)IDT::DefaultInterruptHandler); 
	}
	/* 初始化INT 0 - 31号异常 */
	IDT::SetTrapGate(0, (unsigned long)Exception::DivideErrorEntrance);
	IDT::SetTrapGate(1, (unsigned long)Exception::DebugEntrance);
	IDT::SetTrapGate(2, (unsigned long)Exception::NMIEntrance);
	IDT::SetTrapGate(3, (unsigned long)Exception::BreakpointEntrance);
	IDT::SetTrapGate(4, (unsigned long)Exception::OverflowEntrance);
	IDT::SetTrapGate(5, (unsigned long)Exception::BoundEntrance);
	IDT::SetTrapGate(6, (unsigned long)Exception::InvalidOpcodeEntrance);
	IDT::SetTrapGate(7, (unsigned long)Exception::DeviceNotAvailableEntrance);
	IDT::SetTrapGate(8, (unsigned long)Exception::DoubleFaultEntrance);
	IDT::SetTrapGate(9, (unsigned long)Exception::CoprocessorSegmentOverrunEntrance);
	IDT::SetTrapGate(10,(unsigned long)Exception::InvalidTSSEntrance);
	IDT::SetTrapGate(11,(unsigned long)Exception::SegmentNotPresentEntrance);
	IDT::SetTrapGate(12,(unsigned long)Exception::StackSegmentErrorEntrance);
	IDT::SetTrapGate(13,(unsigned long)Exception::GeneralProtectionEntrance);
	
	/* 缺页异常(INT 14) UNIX V6++中对整个进程图像换入换出，非页式交换，因此不需要缺页异常处理函数 */
	IDT::SetTrapGate(14,(unsigned long)Exception::PageFaultEntrance);
	/* Intel保留异常(INT 15)  使用IDT::DefaultExceptionHandler() */
	IDT::SetTrapGate(16,(unsigned long)Exception::CoprocessorErrorEntrance);
	IDT::SetTrapGate(17,(unsigned long)Exception::AlignmentCheckEntrance);
	IDT::SetTrapGate(18,(unsigned long)Exception::MachineCheckEntrance);
	IDT::SetTrapGate(19,(unsigned long)Exception::SIMDExceptionEntrance);

	/* INT 20 - 31号异常为Intel保留未使用的异常 */

	/* 设置时钟中断的中断门 */
	IDT::SetInterruptGate(0x20, (unsigned long)Time::TimeInterruptEntrance);
	/* 设置键盘中断的中断门 */
	IDT::SetInterruptGate(0x21, (unsigned long)KeyboardInterrupt::KeyboardInterruptEntrance);
	/* 设置IDT中磁盘中断对应中断门 */
	IDT::SetInterruptGate(0x2E, (unsigned long)DiskInterrupt::DiskInterruptEntrance);
	/* 0x80号中断向量作为系统调用，设置系统调用对应的陷入门 */
	IDT::SetTrapGate(0x80, (unsigned long)SystemCall::SystemCallEntrance);
	/* 8259A主片的irq7引脚会产生的未知中断，提供中断处理函数“忽略它” */
	IDT::SetInterruptGate(0x27, (unsigned long)MasterIRQ7);
}

void Machine::InitGDT()
{
	GDT::Initialize();
    TaskStateSegment::Initialize();
}

void Machine::InitPageDirectory()
{
	/* 
	 * 实现操作系统的页表映射:
	 * 物理内存0x00000000-0x00400000(0-4M)将被映射到线性地址
	 * 0x00000000-0x00400000 和 0xC0000000-0xC0400000
	 */
	PageDirectory* pPageDirectory = (PageDirectory*)(PAGE_DIRECTORY_BASE_ADDRESS + KERNEL_SPACE_START_ADDRESS);
	
	/* 填写页目录（0x200#页表）的第0项，使线性地址0-4M映射到物理内存0-4M */
	/*
	pPageDirectory->m_Entrys[0].m_UserSupervisor = 1;                   //用户态
	pPageDirectory->m_Entrys[0].m_Present = 1;
	pPageDirectory->m_Entrys[0].m_ReadWriter = 1;
	pPageDirectory->m_Entrys[0].m_PageTableBaseAddress = KERNEL_PAGE_TABLE_BASE_ADDRESS >> 12;
	*/

	/* 填写页目录（0x200#）页表的第768项，使线性地址0xC0000000-0xC0400000映射到物理内存0-4M。未来核心态空间尺寸大于4M字节，记得这里要改*/
	unsigned int kPageTableIdx = KERNEL_SPACE_START_ADDRESS / PageTable::SIZE_PER_PAGETABLE_MAP; 
	pPageDirectory->m_Entrys[kPageTableIdx].m_UserSupervisor = 0;       // 核心态
	pPageDirectory->m_Entrys[kPageTableIdx].m_Present = 1;
	pPageDirectory->m_Entrys[kPageTableIdx].m_ReadWriter = 1;
	pPageDirectory->m_Entrys[kPageTableIdx].m_PageTableBaseAddress = KERNEL_PAGE_TABLE_BASE_ADDRESS >> 12;


	/* 
	 * 初始化核心态页表。核心态页表被存放在物理地址
	 * 0x200000(2M)，所对应线性地址则为0xC0200000
	 */
	PageTable* pPageTable = (PageTable*)(KERNEL_PAGE_TABLE_BASE_ADDRESS + KERNEL_SPACE_START_ADDRESS);
	/* 
	 * 使用物理内存0-4M填写页表的表项，至此完成物理内存0-4M
	 * 映射到高位0xC0000000-0xC0400000，供操作系统内核使用。
	 */
	for ( unsigned int i = 0; i < PageTable::ENTRY_CNT_PER_PAGETABLE; i++ )
	{
		pPageTable->m_Entrys[i].m_UserSupervisor = 0;
		pPageTable->m_Entrys[i].m_Present = 1;
		pPageTable->m_Entrys[i].m_ReadWriter = 1;
		pPageTable->m_Entrys[i].m_PageBaseAddress = i;
	}


	this->m_PageDirectory = pPageDirectory;
	this->m_KernelPageTable = pPageTable;	

}


#ifdef USE_VESA
void Machine::InitVESAMemoryMap(uintptr_t videoMemAddr, uintptr_t virtualMemAddr, size_t videoMemSize) {
	
	uintptr_t videoMemBegin = videoMemAddr;
	videoMemBegin /= PageTable::SIZE_PER_PAGETABLE_MAP;
	videoMemBegin *= PageTable::SIZE_PER_PAGETABLE_MAP;

	uintptr_t virtualMemBegin = virtualMemAddr - (videoMemAddr - videoMemBegin);
	if (virtualMemBegin % PageTable::SIZE_PER_PAGETABLE_MAP) {
		// todo: panic!
		return;
	}

	uintptr_t videoMemEnd = (videoMemAddr + videoMemSize + PageTable::SIZE_PER_PAGETABLE_MAP - 1);
	videoMemEnd /= PageTable::SIZE_PER_PAGETABLE_MAP;
	videoMemEnd *= PageTable::SIZE_PER_PAGETABLE_MAP;
	

	PageDirectory* pageDir = (PageDirectory*) (PAGE_DIRECTORY_BASE_ADDRESS + KERNEL_SPACE_START_ADDRESS);
	for (uintptr_t addr = videoMemBegin; addr < videoMemEnd; addr += PageTable::SIZE_PER_PAGETABLE_MAP) {
		uintptr_t vAddr = addr + virtualMemBegin - videoMemBegin;
		auto& entry = pageDir->m_Entrys[vAddr / PageTable::SIZE_PER_PAGETABLE_MAP];
		entry.m_UserSupervisor = 0;
		entry.m_Present = 1;
		entry.m_ReadWriter = 1;
		entry.m_PageSize = 1;

		entry.m_PageTableBaseAddress = addr >> 12;
	}


	__asm (
        "mov %%eax, %%cr3"
        :
        : "a" (Machine::PAGE_DIRECTORY_BASE_ADDRESS)
    );
}
#endif



void Machine::InitUserPageTable()
{
	PageDirectory* pPageDirectory = this->m_PageDirectory;
	PageTable* pUserPageTable = 
		(PageTable*)(USER_PAGE_TABLE_BASE_ADDRESS + KERNEL_SPACE_START_ADDRESS);
	unsigned int idx = USER_PAGE_TABLE_BASE_ADDRESS >> 12;
	
	for ( unsigned int j = 0; j < USER_PAGE_TABLE_CNT; j++, idx++ )
	{
		pPageDirectory->m_Entrys[j].m_UserSupervisor = 1;
		pPageDirectory->m_Entrys[j].m_Present = 1;
		pPageDirectory->m_Entrys[j].m_ReadWriter = 1;
		/* 
		 * 页目录项BaseAddress字段中记录页表的物理起始地址，而非线性地址。
		 * 也就是说，分页机制中经由页目录项BaseAddress字段找下一级页表是
		 * 根据页表的物理地址找到它。分页机制的运作不依赖分页机制的本身--对线性地址的解析。
		 */
		pPageDirectory->m_Entrys[j].m_PageTableBaseAddress = idx;
		
		for ( unsigned int i = 0; i < PageTable::ENTRY_CNT_PER_PAGETABLE; i++ )
		{
			pUserPageTable[j].m_Entrys[i].m_UserSupervisor = 1;
			pUserPageTable[j].m_Entrys[i].m_Present = 1;
			pUserPageTable[j].m_Entrys[i].m_ReadWriter = 1;
			pUserPageTable[j].m_Entrys[i].m_PageBaseAddress = 0x00000 + i +j * 1024;
		}
	}

	this->m_UserPageTable = pUserPageTable;	
}

void Machine::EnablePageProtection()
{
	/* 
	 * pageDirBaseAddr是在高位内核空间的线性地址，需要转换为物理地址。
	 * PhysicalAddress = LinearAddress - 0xC0000000
	 */
	unsigned int pageDirBaseAddr = (unsigned int)(&GetPageDirectory());
	unsigned int pageDirPhyBaseAddr = pageDirBaseAddr - Machine::KERNEL_SPACE_START_ADDRESS;
	
	/* 寄存器CR3中写入页目录起始物理地址，CR0的PG位置1，开启分页机制 */
	__asm__ __volatile__("	movl %0, %%cr3;		\
							movl %%cr0, %%eax;	\
							orl $0x80000000, %%eax;	\
							movl %%eax, %%cr0" : : "a"(pageDirPhyBaseAddr) );
}

PageDirectory& Machine::GetPageDirectory()
{
	return *(this->m_PageDirectory);
}

PageTable& Machine::GetKernelPageTable()
{
	return *(this->m_KernelPageTable);
}

PageTable* Machine::GetUserPageTableArray()
{
	return this->m_UserPageTable;
}
