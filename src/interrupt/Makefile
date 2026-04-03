include-path := $(abs_srctree)/src $(abs_srctree)/src/include

objs := DiskInterrupt.o Exception.o KeyboardInterrupt.o \
	SystemCall.o TimeInterrupt.o

include $(abs_srctree)/scripts/Makefile

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-use-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH
