#ifndef UITILITY_H
#define UITILITY_H

#include "sys/types.h"

/*
 *@comment 定义一些工具常量
 * 由于使用了编译选项-fno-builtin，
 * 编译器不提供这些常量的定义。
 */
#define NULL	0

/* 保存系统时间信息的结构体 */
struct SystemTime
{
	int Second;		/* Seconds: 0 ~ 59 */
	int Minute;		/* Minutes: 0 ~ 59 */
	int Hour;		/* Hours of Day: 0 ~ 23 */
	int DayOfMonth;	/* Day of Month: 1 ~ 31 */
	int Month;		/* Months since January: 1 ~ 12 */
	int Year;		/* Years since 1900 */
	int DayOfWeek;	/* Days since Sunday: 1 ~ 7 */
};

/* 记录进程使用的内核态和用户态下CPU时间的结构体 */
struct tms
{
	int utime;		/* 进程用户态CPU时间 */
	int stime;		/* 进程核心态CPU时间 */
	int cutime;		/* 子进程用户态时间总和 */
	int cstime;		/* 子进程核心态时间总和 */
};

extern "C" {
void _mem_copy(unsigned long src, unsigned long dst, unsigned int count);
int _calculate_page_need(unsigned int memory_need, unsigned int page_size);
void _copy_seg(unsigned long src, unsigned long dst);
void _copy_seg2(unsigned long src, unsigned long dst);
short _get_major(short dev);
short _get_minor(short dev);
short _set_major(short dev, short value);
short _set_minor(short dev, short value);
void Utility_panic(const char* str);
void _dword_copy(const int* src, int* dst, int count);
int _min(int a, int b);
int _max(int a, int b);
int _bcd_to_binary(int value);
void _io_move(const unsigned char* from, unsigned char* to, int count);
unsigned int _make_kernel_time(const SystemTime* time);
bool _is_leap_year(int year);
unsigned int _days_in_year(int year);
}

/*
 *@comment 一些经常被使用到的工具函数
 *
 *
 */
class Utility
{
public:
	static void MemCopy(unsigned long src, unsigned long des, unsigned int count) {
        _mem_copy(src, des, count);
    }

	static int CaluPageNeed(unsigned int memoryneed, unsigned int pagesize) {
        return _calculate_page_need(memoryneed, pagesize);
    }

#if 0  // use libyrosstd instead
	static void StringCopy(char* src, char* dst);

	static int StringLength(char* pString);
#endif

	/* @comment
	 * 用于从物理地址src copy 到物理地址des 1个byte
	 */
	static void CopySeg(unsigned long src, unsigned long des) {
        _copy_seg(src, des);
    }
	static void CopySeg2(unsigned long src, unsigned long des) {
        _copy_seg2(src, des);
    }
	/* 提取参数dev中的主设备号major，高8比特 */
	static short GetMajor(const short dev) {
        return _get_major(dev);
    }
	/* 提取参数dev中的次设备号minor，低8比特 */
	static short GetMinor(const short dev) {
        return _get_minor(dev);
    }
	/* 设置参数dev中的主设备号部分，高8比特 */
	static short SetMajor(short dev, const short value) {
        return _set_major(dev, value);
    }
	/* 设置参数dev中的次设备号部分，低8比特 */
	static short SetMinor(short dev, const short value) {
        return _set_minor(dev, value);
    }
	/* 输出错误信息，然后死循环 */
	static void Panic(const char* str) {
		return Utility_panic(str);
	}

	/* 以src为源地址，dst为目的地址，复制count个双字 */
	static void DWordCopy(int* src, int* dst, int count) {
        _dword_copy(src, dst, count);
    }

	static int Min(int a, int b) {
        return _min(a, b);
    }

	static int Max(int a, int b) {
        return _max(a, b);
    }

	/* Convert BCD to Binary */
	static int BCDToBinary(int value) {
        return _bcd_to_binary(value);
    }

	/* 用于在读、写文件时，高速缓存与用户指定目标内存区域之间数据传送 */
	static void IOMove(unsigned char* from, unsigned char* to, int count) {
        _io_move(from, to, count);
    }

	/* 根据SystemTime结构体中的值计算出内核格式的时间值：从1970年1月1日0时至当前的秒数 */
	static unsigned int MakeKernelTime(struct SystemTime* pTime) {
        return _make_kernel_time(pTime);
    }

	static bool IsLeapYear(int year) {
        return _is_leap_year(year);
    }

	static unsigned int DaysInYear(int year) {
        return _days_in_year(year);
    }

	/* 某个月份前经过的天数 */
	constexpr static const unsigned int DaysBeforeMonth[13] = {0xFFFFFFFF/* Unused */, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334};
};

#endif
