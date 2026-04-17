#include "BlockDevice.h"

extern "C" int block_device_open(short dev, int mode);
extern "C" int block_device_close(short dev, int mode);
extern "C" int block_device_strategy(Buf* bp);
extern "C" void block_device_start(short major);

BlockDevice::BlockDevice()
{
}

BlockDevice::~BlockDevice()
{
}

int BlockDevice::Open(short dev, int mode)
{
	return block_device_open(dev, mode);
}

int BlockDevice::Close(short dev, int mode)
{
	return block_device_close(dev, mode);
}

int BlockDevice::Strategy(Buf *bp)
{
	return block_device_strategy(bp);
}

void BlockDevice::Start()
{
	block_device_start(0);
}

ATABlockDevice::ATABlockDevice() {
}

ATABlockDevice::~ATABlockDevice()
{
}

ATABlockDevice g_ATADevice;

int ATABlockDevice::Open(short dev, int mode)
{
	return block_device_open(dev, mode);
}

int ATABlockDevice::Close(short dev, int mode)
{
	return block_device_close(dev, mode);
}

int ATABlockDevice::Strategy(Buf* bp)
{
	return block_device_strategy(bp);
}

void ATABlockDevice::Start()
{
	block_device_start(0);
}
