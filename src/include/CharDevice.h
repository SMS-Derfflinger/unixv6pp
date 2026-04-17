#ifndef CHAR_DEVICE_H
#define CHAR_DEVICE_H

#include "TTy.h"

#ifdef __cplusplus
extern "C" {
#endif

int char_device_open(short dev, int mode);
int char_device_close(short dev, int mode);
int char_device_read(short dev, unsigned char* out, int count);
int char_device_write(short dev, const unsigned char* data, int count);
void rust_tty_input_byte(unsigned char ch);
void rust_tty_flush();

#ifdef __cplusplus
}
#endif

class CharDevice
{
public:
	CharDevice();
	virtual ~CharDevice();
	/* 
	 * 定义为虚函数，由派生类进行override实现设备
	 * 特定操作。正常情况下，基类中函数不应被调用到。
	 */
	virtual void Open(short dev, int mode) = 0;
	virtual void Close(short dev, int mode) = 0;
	virtual void Read(short dev) = 0;
	virtual void Write(short dev) = 0;
	virtual void SgTTy(short dev, TTy* pTTy) = 0;
};


class ConsoleDevice : public CharDevice
{
public:
	ConsoleDevice();
	virtual ~ConsoleDevice();
	/* 
	 * Override基类CharDevice中的虚函数，实现
	 * 派生类ConsoleDevice特定的设备操作逻辑。
	 */
	void Open(short dev, int mode);
	void Close(short dev, int mode);
	void Read(short dev);
	void Write(short dev);
	void SgTTy(short dev, TTy* pTTy);
};

#endif
