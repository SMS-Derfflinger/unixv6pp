#ifndef MEMORY_DESCRIPTOR_H
#define MEMORY_DESCRIPTOR_H

#include "PageTable.h"

class MemoryDescriptor
{
public:
	/* 用户空间大小 8M 0x0 - 0x800000 2 PageTable */
	static const unsigned int USER_SPACE_SIZE	= 0x800000; 
	static const unsigned int USER_SPACE_PAGE_TABLE_CNT = 0x2;
	static const unsigned long USER_SPACE_START_ADDRESS		= 0x0;



public:
	MemoryDescriptor();
	~MemoryDescriptor();

public:
	/* 申请并初始化PageDirectory，在做Map操作前使用 */
	void Initialize();
	/* 在释放进程时，需要调用该操作释放被占用的页表 */
	void Release();

private:

	/* @comment设置页表目录项
	 * @param
	 * unsigned long virtualAddress:	虚拟地址(以字节为单位) 
	 * unsigned int size:				需要映射的虚拟地址大小(以字节为单位) 
	 * unsigned long phyPageIdx:		其实物理页索引号(页为单位)		
	 * bool isReadWrite:				页属性，true为可读可写页
	 */
	unsigned int MapEntry(unsigned long virtualAddress, unsigned int size, unsigned long phyPageIdx, bool isReadWrite);

public:
	PageTable*		m_UserPageTableArray;
	/* 以下数据都是线性地址 */
	unsigned long	m_TextStartAddress;	/* 代码段起始地址 */
	unsigned long	m_TextSize;			/* 代码段长度 */

	unsigned long	m_DataStartAddress; /* 数据段起始地址 */
	unsigned long	m_DataSize;			/* 数据段长度 */

	unsigned long	m_StackSize;		/* 栈段长度 */
	//unsigned long	m_HeapSize;			/* 堆段长度 */
};

#endif

