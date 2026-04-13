include-path := $(abs_srctree)/src $(abs_srctree)/src/include $(abs_srctree)/fs

objs := ATADriver.o BlockDevice.o BufferManager.o CharDevice.o \
	DeviceManager.o DMA.o

include $(abs_srctree)/scripts/Makefile

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-use-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH
