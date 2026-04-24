#include "Kernel.h"
#include "CharDevice.h"
#include "Video.h"
#include "Utility.h"
#include "Regs.h"

Kernel Kernel::instance;

/*
 * 交换区相关全局manager
 */
SwapperManager g_SwapperManager(&(Allocator::GetInstance()));

ConsoleDevice g_ConsoleDevice;

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

extern "C" int cpp_swapper_manager_initialize()
{
	return g_SwapperManager.Initialize();
}

SwapperManager& Kernel::GetSwapperManager()
{
	return *(this->m_SwapperManager);
}

User& Kernel::GetUser()
{
	return *(User*)USER_ADDRESS;
}
