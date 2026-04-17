#include "CharDevice.h"
#include "Kernel.h"
#include "Utility.h"

extern "C" void rust_process_sleep(unsigned long chan, int pri) {
    User_get_procp()->Sleep(chan, pri);
}

extern "C" void rust_process_wakeup_all(unsigned long chan) {
    Kernel::Instance().GetProcessManager().WakeUpAll(chan);
}

/*==============================class CharDevice===============================*/
CharDevice::CharDevice() {
}

CharDevice::~CharDevice() {
    // nothing to do here
}

void CharDevice::Open(short dev, int mode) {
    Utility::Panic("ERROR! Base Class: CharDevice::Open()!");
}

void CharDevice::Close(short dev, int mode) {
    Utility::Panic("ERROR! Base Class: CharDevice::Close()!");
}

void CharDevice::Read(short dev) {
    Utility::Panic("ERROR! Base Class: CharDevice::Read()!");
}

void CharDevice::Write(short dev) {
    Utility::Panic("ERROR! Base Class: CharDevice::Write()!");
}

void CharDevice::SgTTy(short dev, TTy *pTTy) {
    Utility::Panic("ERROR! Base Class: CharDevice::SgTTy()!");
}

/*==============================class ConsoleDevice===============================*/
ConsoleDevice g_ConsoleDevice;
extern TTy g_TTy;

ConsoleDevice::ConsoleDevice() {
    // nothing to do here
}

ConsoleDevice::~ConsoleDevice() {
    // nothing to do here
}

void ConsoleDevice::Open(short dev, int mode) {
    short minor = Utility::GetMinor(dev);
    User &u = Kernel::Instance().GetUser();
    int result;

    if (minor != 0) {
        return;
    }

    if (NULL == User_get_procp()->p_ttyp) {
        User_get_procp()->p_ttyp = &g_TTy;
    }

    result = char_device_open(dev, mode);
    if (result < 0) {
        User_get_error() = (User::ErrorCode)(-result);
    }
}

void ConsoleDevice::Close(short dev, int mode) {
    int result = char_device_close(dev, mode);
    if (result < 0) {
        User_get_error() = (User::ErrorCode)(-result);
    }
}

void ConsoleDevice::Read(short dev) {
    short minor = Utility::GetMinor(dev);
    User &u = Kernel::Instance().GetUser();
    int result;

    if (0 == minor) {
        result = char_device_read(dev, User_get_IOParam().m_Base, User_get_IOParam().m_Count);
        if (result < 0) {
            User_get_error() = (User::ErrorCode)(-result);
            return;
        }
        User_get_IOParam().m_Base += result;
        User_get_IOParam().m_Count -= result;
    }
}

void ConsoleDevice::Write(short dev) {
    short minor = Utility::GetMinor(dev);
    User &u = Kernel::Instance().GetUser();
    int result;

    if (0 == minor) {
        result = char_device_write(dev, User_get_IOParam().m_Base, User_get_IOParam().m_Count);
        if (result < 0) {
            User_get_error() = (User::ErrorCode)(-result);
            return;
        }
        User_get_IOParam().m_Base += result;
        User_get_IOParam().m_Count -= result;
    }
}

void ConsoleDevice::SgTTy(short dev, TTy *pTTy) {
}
