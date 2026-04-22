#include "Kernel.h"
#include "Video.h"
#include "Utility.h"
#include "Regs.h"

Kernel Kernel::instance;

/* 
 * 内存管理相关的全局manager
 */
UserPageManager g_UserPageManager;
KernelPageManager g_KernelPageManager;

/*
 * 交换区相关全局manager
 */
SwapperManager g_SwapperManager(&(Allocator::GetInstance()));

/* 
 * 进程相关全局manager
 */
ProcessManager g_ProcessManager;

/*
 * 设备管理、高速缓存管理全局manager
 */
BufferManager g_BufferManager;
DeviceManager g_DeviceManager;

/*
 * 文件系统相关全局manager
 */
FileSystem g_FileSystem;
FileManager g_FileManager;

Kernel::Kernel()
{
}

Kernel::~Kernel()
{
}

Kernel& Kernel::Instance()
{
	return Kernel::instance;
}

extern "C" void cpp_exception_handle(struct pt_context* context, int signal, const char* message)
{
	Process* current = User_get_procp();

	if ( (context->xcs & USER_MODE) == USER_MODE )
	{
		current->PSignal(signal);
		if ( current->IsSig() )
		{
			current->PSig(context);
		}
	}
	else
	{
		Utility::Panic(message);
	}
}

extern "C" void cpp_exception_page_fault(struct pt_regs* regs, struct pte_context* context)
{
	(void)regs;

	Process* current = User_get_procp();
	MemoryDescriptor& md = User_get_MemoryDescriptor();

	unsigned int cr2;
	__asm__ __volatile__(" mov %%cr2, %0":"=r"(cr2) );

	if ( (context->xcs & USER_MODE) == USER_MODE )
	{
		if ( cr2 < MemoryDescriptor::USER_SPACE_SIZE - md.m_StackSize && cr2 >= context->esp - 8
				&& md.m_DataSize + md.m_StackSize + PAGE_SIZE < MemoryDescriptor::USER_SPACE_SIZE - md.m_DataStartAddress )
		{
			current->SStack();
		}
		else
		{
			Diagnose::Write("Invalid MM access");
			current->PSignal(User::SIGSEGV);
			if ( current->IsSig() )
			{
				current->PSig((pt_context *)&context->eip);
			}
		}
	}
	else
	{
		Diagnose::Write("at eip=0x%x cr2=0x%x, ", context->eip, cr2);
		Utility::Panic("Page Fault in Kernel Mode.");
	}
}

void Kernel::InitMemory()
{
	this->m_KernelPageManager = &g_KernelPageManager;
	this->m_UserPageManager = &g_UserPageManager;

	Diagnose::Write("Initilize Memory...");
	Diagnose::Write("Ok.\n");

	this->m_SwapperManager = &g_SwapperManager;
	Diagnose::Write("Initialize Swapper...");
	this->GetSwapperManager().Initialize();
	Diagnose::Write("Ok.\n");

}

void Kernel::InitProcess()
{
	this->m_ProcessManager = &g_ProcessManager;

	Diagnose::Write("Initilize Process...");
	this->GetProcessManager().Initialize();
	Diagnose::Write("Ok.\n");
}

void Kernel::InitBuffer()
{
	this->m_BufferManager = &g_BufferManager;
	this->m_DeviceManager = &g_DeviceManager;

	Diagnose::Write("Initialize Buffer...");
	this->GetBufferManager().Initialize();
	Diagnose::Write("OK.\n");

	Diagnose::Write("Initialize Device Manager...");
	this->GetDeviceManager().Initialize();
	Diagnose::Write("OK.\n");
}

void Kernel::InitFileSystem()
{
	this->m_FileSystem = &g_FileSystem;
	this->m_FileManager = &g_FileManager;

	Diagnose::Write("Initialize File System...");
	this->GetFileSystem().Initialize();
	Diagnose::Write("OK.\n");

	Diagnose::Write("Initialize File Manager...");
	this->GetFileManager().Initialize();
	Diagnose::Write("OK.\n");
}

extern "C" int init_serial();

void Kernel::Initialize()
{
	init_serial();
	InitMemory();
	InitProcess();
	InitBuffer();
	InitFileSystem();
}

KernelPageManager& Kernel::GetKernelPageManager()
{
	return *(this->m_KernelPageManager);
}

UserPageManager& Kernel::GetUserPageManager()
{
	return *(this->m_UserPageManager);
}

ProcessManager& Kernel::GetProcessManager()
{
	return *(this->m_ProcessManager);
}

SwapperManager& Kernel::GetSwapperManager()
{
	return *(this->m_SwapperManager);
}

BufferManager& Kernel::GetBufferManager()
{
	return *(this->m_BufferManager);
}

DeviceManager& Kernel::GetDeviceManager()
{
	return *(this->m_DeviceManager);
}

FileSystem& Kernel::GetFileSystem()
{
	return *(this->m_FileSystem);
}

FileManager& Kernel::GetFileManager()
{
	return *(this->m_FileManager);
}

User& Kernel::GetUser()
{
	return *(User*)USER_ADDRESS;
}
