include-path := $(abs_srctree)/src $(abs_srctree)/src/include

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH

objs := ATADriver.o BlockDevice.o BufferManager.o CharDevice.o \
	DeviceManager.o DMA.o

include $(abs_srctree)/scripts/Makefile
