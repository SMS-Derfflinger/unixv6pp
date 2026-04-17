#include "DeviceManager.h"

extern ATABlockDevice g_ATADevice;
extern ConsoleDevice g_ConsoleDevice;

extern "C" void device_manager_initialize();
extern "C" int device_manager_n_block_devices();
extern "C" int device_manager_n_char_devices();
extern "C" void device_manager_require_block_device(short major);
extern "C" void device_manager_require_char_device(short major);

DeviceManager::DeviceManager()
{
}

DeviceManager::~DeviceManager()
{
}

void DeviceManager::Initialize()
{
	device_manager_initialize();
}

int DeviceManager::GetNBlkDev()
{
	return device_manager_n_block_devices();
}

BlockDevice& DeviceManager::GetBlockDevice(short major)
{
	device_manager_require_block_device(major);
	return g_ATADevice;
}

int DeviceManager::GetNChrDev()
{
	return device_manager_n_char_devices();
}

CharDevice& DeviceManager::GetCharDevice(short major)
{
	device_manager_require_char_device(major);
	return g_ConsoleDevice;
}
