#ifndef CHAR_DEVICE_H
#define CHAR_DEVICE_H

#include "TTy.h"

class CharDevice
{
public:
	CharDevice() {}
	virtual ~CharDevice() {}
	virtual void SgTTy(short dev, TTy* pTTy) = 0;
};

class ConsoleDevice : public CharDevice
{
public:
	ConsoleDevice() {}
	~ConsoleDevice() {}
	void SgTTy(short dev, TTy* pTTy) {
        
    }
};

extern ConsoleDevice g_ConsoleDevice;

#endif
