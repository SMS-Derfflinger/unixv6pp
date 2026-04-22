#include "CharDevice.h"
#include "Kernel.h"
#include "Utility.h"

// fuck
extern TTy g_TTy;

extern "C" void char_device_fuck_tty() {
    if (NULL == User_get_procp()->p_ttyp) {
        User_get_procp()->p_ttyp = &g_TTy;
    }
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
    char_device_open(dev, mode);
}

void ConsoleDevice::Close(short dev, int mode) {
    char_device_close(dev, mode);
}

void ConsoleDevice::Read(short dev) {
    char_device_read(dev);
}

void ConsoleDevice::Write(short dev) {
    char_device_write(dev);
}

void ConsoleDevice::SgTTy(short dev, TTy *pTTy) {
}
