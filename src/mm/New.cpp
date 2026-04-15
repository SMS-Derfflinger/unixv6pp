#include "New.h"
#include "Utility.h"

void* operator new(unsigned int size)
{
	Utility::Panic("operator new called\n");
	return nullptr;
}

void* operator new(__SIZE_TYPE__ size, void* ptr)
{
	return ptr;
}

void operator delete(void* p)
{
	Utility::Panic("operator delete called\n");
}

void operator delete(void* p, unsigned int n)
{
	Utility::Panic("operator delete called\n");
}
