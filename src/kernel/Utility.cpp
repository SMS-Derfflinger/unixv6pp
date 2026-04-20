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
