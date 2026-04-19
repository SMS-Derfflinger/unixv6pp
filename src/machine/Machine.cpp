#include "Machine.h"
#include "Exception.h"
#include "TimeInterrupt.h"
#include "DiskInterrupt.h"
#include "KeyboardInterrupt.h"
#include "SystemCall.h"

#include "PageManager.h"

Machine Machine::instance;

extern "C" {
struct MachineIDTHandlers
{
	unsigned int divide_error;
	unsigned int debug;
	unsigned int nmi;
	unsigned int breakpoint;
	unsigned int overflow;
	unsigned int bound;
	unsigned int invalid_opcode;
	unsigned int device_not_available;
	unsigned int double_fault;
	unsigned int coprocessor_segment_overrun;
	unsigned int invalid_tss;
	unsigned int segment_not_present;
	unsigned int stack_segment_error;
	unsigned int general_protection;
	unsigned int page_fault;
	unsigned int coprocessor_error;
	unsigned int alignment_check;
	unsigned int machine_check;
	unsigned int simd_exception;
	unsigned int time;
	unsigned int keyboard;
	unsigned int disk;
	unsigned int system_call;
	unsigned int master_irq7;
};

void _load_idt();
void _load_gdt();
void _load_task_register();
void _enable_page_protection(const void* page_directory);
void _flush_page_directory();
void _init_idt(const MachineIDTHandlers* handlers);
void _init_gdt();
}

Machine& Machine::Instance()
{
	return instance;
}

void Machine::LoadIDT()
{
	_load_idt();
}

void Machine::LoadGDT()
{
	_load_gdt();
}

void Machine::LoadTaskRegister()
{
	_load_task_register();
}

extern "C" void MasterIRQ7();

void Machine::InitIDT()
{
	MachineIDTHandlers handlers = {
		(unsigned int)Exception::DivideErrorEntrance,
		(unsigned int)Exception::DebugEntrance,
		(unsigned int)Exception::NMIEntrance,
		(unsigned int)Exception::BreakpointEntrance,
		(unsigned int)Exception::OverflowEntrance,
		(unsigned int)Exception::BoundEntrance,
		(unsigned int)Exception::InvalidOpcodeEntrance,
		(unsigned int)Exception::DeviceNotAvailableEntrance,
		(unsigned int)Exception::DoubleFaultEntrance,
		(unsigned int)Exception::CoprocessorSegmentOverrunEntrance,
		(unsigned int)Exception::InvalidTSSEntrance,
		(unsigned int)Exception::SegmentNotPresentEntrance,
		(unsigned int)Exception::StackSegmentErrorEntrance,
		(unsigned int)Exception::GeneralProtectionEntrance,
		(unsigned int)Exception::PageFaultEntrance,
		(unsigned int)Exception::CoprocessorErrorEntrance,
		(unsigned int)Exception::AlignmentCheckEntrance,
		(unsigned int)Exception::MachineCheckEntrance,
		(unsigned int)Exception::SIMDExceptionEntrance,
		(unsigned int)Time::TimeInterruptEntrance,
		(unsigned int)KeyboardInterrupt::KeyboardInterruptEntrance,
		(unsigned int)DiskInterrupt::DiskInterruptEntrance,
		(unsigned int)SystemCall::SystemCallEntrance,
		(unsigned int)MasterIRQ7
	};

	_init_idt(&handlers);
}

void Machine::InitGDT()
{
	_init_gdt();
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
	pPageDirectory->m_Entrys[kPageTableIdx].m_UserSupervisor = 0;       // ����̬
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


	_flush_page_directory();
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
	_enable_page_protection(&GetPageDirectory());
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
