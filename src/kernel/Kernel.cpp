#include "Kernel.h"

Kernel Kernel::instance;

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

User& Kernel::GetUser()
{
	return *(User*)USER_ADDRESS;
}
