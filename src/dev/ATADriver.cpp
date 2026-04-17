#include "ATADriver.h"
#include "BufferManager.h"
#include "Utility.h"
#include "IOPort.h"
#include "Kernel.h"
#include "Chip8259A.h"

extern "C" void rust_ata_handler();
extern "C" void rust_ata_dev_start(struct Buf* bp);

extern "C" Devtab* ata_driver_current_devtab()
{
	short major = Utility::GetMajor(DeviceManager::ROOTDEV);
	BlockDevice& bdev = Kernel::Instance().GetDeviceManager().GetBlockDevice(major);
	return bdev.d_tab;
}

extern "C" void ata_driver_start_current()
{
	short major = Utility::GetMajor(DeviceManager::ROOTDEV);
	BlockDevice& bdev = Kernel::Instance().GetDeviceManager().GetBlockDevice(major);
	bdev.Start();
}

extern "C" void ata_driver_io_done(struct Buf* bp)
{
	Kernel::Instance().GetBufferManager().IODone(bp);
}

extern "C" void ata_driver_send_eoi()
{
	IOPort::OutByte(Chip8259A::MASTER_IO_PORT_1, Chip8259A::EOI);
	IOPort::OutByte(Chip8259A::SLAVE_IO_PORT_1, Chip8259A::EOI);
}

void ATADriver::ATAHandler(struct pt_regs *reg, struct pt_context *context)
{
	(void)reg;
	(void)context;
	rust_ata_handler();
}

void ATADriver::DevStart(struct Buf* bp)
{
	rust_ata_dev_start(bp);
}
