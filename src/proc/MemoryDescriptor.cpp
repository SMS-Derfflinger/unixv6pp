#include "MemoryDescriptor.h"
#include "Kernel.h"
#include "PageManager.h"
#include "Machine.h"
#include "PageDirectory.h"
#include "Video.h"

MemoryDescriptor::MemoryDescriptor() {}
MemoryDescriptor::~MemoryDescriptor() {}

PageTable* MemoryDescriptor::GetUserPageTableArray()
{
	return this->m_UserPageTableArray;
}
unsigned long MemoryDescriptor::GetTextStartAddress()
{
	return this->m_TextStartAddress;
}
unsigned long MemoryDescriptor::GetTextSize()
{
	return this->m_TextSize;
}
unsigned long MemoryDescriptor::GetDataStartAddress()
{
	return this->m_DataStartAddress;
}
unsigned long MemoryDescriptor::GetDataSize()
{
	return this->m_DataSize;
}
unsigned long MemoryDescriptor::GetStackSize()
{
	return this->m_StackSize;
}
