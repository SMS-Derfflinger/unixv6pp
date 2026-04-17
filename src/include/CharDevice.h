#ifndef CHAR_DEVICE_H
#define CHAR_DEVICE_H

#include "TTy.h"

#ifdef __cplusplus
extern "C" {
#endif

void char_device_open(short dev, int mode);
void char_device_close(short dev, int mode);
void char_device_read(short dev);
void char_device_write(short dev);
void tty_input_byte(unsigned char ch);
void tty_flush();

#ifdef __cplusplus
}
#endif

class CharDevice
{
public:
	CharDevice();
	virtual ~CharDevice();

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

	void Open(short dev, int mode);
	void Close(short dev, int mode);
	void Read(short dev);
	void Write(short dev);
	void SgTTy(short dev, TTy* pTTy);
};

#endif
