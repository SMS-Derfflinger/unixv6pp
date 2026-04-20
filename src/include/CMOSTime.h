#ifndef CMOSTIME_H
#define CMOSTIME_H

#include "Utility.h"

extern "C" void _cmos_read_time(struct SystemTime* pTime);
extern "C" int _cmos_read_byte_low();
extern "C" int _cmos_read_byte_high();

class CMOSTime
{
public:
	/* 从COMS存储器中获取系统时间，填充SystemTime结构 */
	static void ReadCMOSTime(struct SystemTime* pTime) {
        _cmos_read_time(pTime);
    }

	/* 读取指定偏移位置上的CMOS存储器数据内容 */
	static int ReadCMOSByteLow() {
        return _cmos_read_byte_low();
    }

    static int ReadCMOSByteHigh() {
        return _cmos_read_byte_high();
    }
};

#endif
