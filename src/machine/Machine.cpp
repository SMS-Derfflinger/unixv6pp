#include "Machine.h"

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
void Machine_init_idt();
void _init_gdt();
void _init_page_directory();
void _init_user_page_table();
void _init_vesa_memory_map(uintptr_t video_memory_address, uintptr_t virtual_memory_address, size_t video_memory_size);
void* _page_directory();
void* _kernel_page_table();
void* _user_page_table_array();
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
extern "C" void DiskInterruptEntrance();
extern "C" void KeyboardInterruptEntrance();

void Machine::InitIDT()
{
	Machine_init_idt();
}

void Machine::InitGDT()
{
	_init_gdt();
}

void Machine::InitPageDirectory()
{
	_init_page_directory();
}

#ifdef USE_VESA
void Machine::InitVESAMemoryMap(uintptr_t videoMemAddr, uintptr_t virtualMemAddr, size_t videoMemSize)
{
	_init_vesa_memory_map(videoMemAddr, virtualMemAddr, videoMemSize);
}
#endif


void Machine::InitUserPageTable()
{
	_init_user_page_table();
}

void Machine::EnablePageProtection()
{
	_enable_page_protection(&GetPageDirectory());
}

PageDirectory& Machine::GetPageDirectory()
{
	return *(PageDirectory*)_page_directory();
}

PageTable& Machine::GetKernelPageTable()
{
	return *(PageTable*)_kernel_page_table();
}

PageTable* Machine::GetUserPageTableArray()
{
	return (PageTable*)_user_page_table_array();
}
