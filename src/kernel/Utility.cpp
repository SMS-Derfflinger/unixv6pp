#include "Utility.h"
#include "Kernel.h"
#include "User.h"
#include "PageManager.h"
#include "Machine.h"
#include "MemoryDescriptor.h"
#include "Video.h"
#include "Assembly.h"

extern "C" {
void _mem_copy(unsigned long src, unsigned long dst, unsigned int count);
int _calculate_page_need(unsigned int memory_need, unsigned int page_size);
short _get_major(short dev);
short _get_minor(short dev);
short _set_major(short dev, short value);
short _set_minor(short dev, short value);
void _dword_copy(const int* src, int* dst, int count);
int _min(int a, int b);
int _max(int a, int b);
int _bcd_to_binary(int value);
void _io_move(const unsigned char* from, unsigned char* to, int count);
unsigned int _make_kernel_time(const SystemTime* time);
bool _is_leap_year(int year);
unsigned int _days_in_year(int year);
}

void Utility::MemCopy(unsigned long src, unsigned long des, unsigned int count)
{
	_mem_copy(src, des, count);
}


int Utility::CaluPageNeed(unsigned int memoryneed, unsigned int pagesize)
{
	return _calculate_page_need(memoryneed, pagesize);
}

#if 0  // use libyrosstd instead
void Utility::StringCopy(char* src, char* dst)
{
	while ( (*dst++ = *src++) != 0 ) ;
}

int Utility::StringLength(char* pString)
{
	int length = 0;
	char* pChar = pString;

	while( *pChar++ )
	{
		length++;
	}

	/* 返回字符串长度 */
	return length;
}
#endif 

void Utility::CopySeg2(unsigned long src, unsigned long des)
{
	PageTableEntry* userPageTable = (PageTableEntry*)Machine::Instance().GetUserPageTableArray();
	

	/*
	 * 先保存原用户态第一页与第二页PageTableEntry，因为下面的操作
	 * 将会将src所在页映射到0#目录表项，des映射到1#表项，最后进行copy
	 */
	unsigned long oriEntry1 = userPageTable[0].m_PageBaseAddress;
	unsigned long oriEntry2 = userPageTable[1].m_PageBaseAddress;	

	userPageTable[0].m_PageBaseAddress = src / PAGE_SIZE;
	userPageTable[1].m_PageBaseAddress = des / PAGE_SIZE;

	unsigned char* addressSrc = (unsigned char*)(src % PAGE_SIZE);	
	//第二页virtual addess从4096开始
	unsigned char* addressDes = (unsigned char*)(PAGE_SIZE + des % PAGE_SIZE);	
	//需要刷新页表缓存
	FlushPageDirectory();

	*addressDes = *addressSrc;
	
	//恢复原页表映射
	userPageTable[0].m_PageBaseAddress = oriEntry1;
	userPageTable[1].m_PageBaseAddress = oriEntry2;
	FlushPageDirectory();
}

void Utility::CopySeg(unsigned long src, unsigned long des)
{
	PageTableEntry* PageTable = Machine::Instance().GetKernelPageTable().m_Entrys;

	/*
	 * 先保存原用户态第一页与第二页PageTableEntry，因为下面的操作
	 * 将会将src所在页映射到0#目录表项，des映射到1#表项，最后进行copy
	 */
	unsigned long oriEntry1 = PageTable[borrowedPTE].m_PageBaseAddress;
	unsigned long oriEntry2 = PageTable[borrowedPTE + 1].m_PageBaseAddress;

	PageTable[256].m_PageBaseAddress = src / PAGE_SIZE;
	PageTable[257].m_PageBaseAddress = des / PAGE_SIZE;

	unsigned char* addressSrc = (unsigned char*)(0xC0000000 + borrowedPTE * PAGE_SIZE + src % PAGE_SIZE);

	unsigned char* addressDes = (unsigned char*)(0xC0000000 + (borrowedPTE + 1) * PAGE_SIZE + des % PAGE_SIZE);
	//需要刷新页表缓存
	FlushPageDirectory();

	*addressDes = *addressSrc;

	//恢复原页表映射
	PageTable[borrowedPTE].m_PageBaseAddress = oriEntry1;
	PageTable[(borrowedPTE + 1)].m_PageBaseAddress = oriEntry2;
	FlushPageDirectory();
}

extern "C" void phys_copy(unsigned long from, unsigned long to, unsigned long len) {
        while (len--)
                Utility::CopySeg(from++, to++);
}

short Utility::GetMajor(const short dev)
{
	return _get_major(dev);
}

short Utility::GetMinor(const short dev)
{
	return _get_minor(dev);
}

short Utility::SetMajor(short dev, const short value)
{
	return _set_major(dev, value);
}

short Utility::SetMinor(short dev, const short value)
{
	return _set_minor(dev, value);
}

void Utility::Panic(const char* str)
{
	Diagnose::TraceOn();
	Diagnose::Write("%s\n", str);
	X86Assembly::CLI();
	for(;;);
}

void Utility::DWordCopy(int *src, int *dst, int count)
{
	_dword_copy(src, dst, count);
}

int Utility::Min(int a, int b)
{
	return _min(a, b);
}

int Utility::Max(int a, int b)
{
	return _max(a, b);
}

int Utility::BCDToBinary( int value )
{
	return _bcd_to_binary(value);
}

void Utility::IOMove(unsigned char* from, unsigned char* to, int count)
{
	_io_move(from, to, count);
}

unsigned int Utility::MakeKernelTime( struct SystemTime* pTime )
{
	return _make_kernel_time(pTime);
}

/* 某个月份前经过的天数，第0项不使用，未纳入计算闰年2月份29天 */
const unsigned int Utility::DaysBeforeMonth[13] = {0xFFFFFFFF/* Unused */, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334};

bool Utility::IsLeapYear( int year )
{
	return _is_leap_year(year);
}

unsigned int Utility::DaysInYear( int year )
{
	return _days_in_year(year);
}
