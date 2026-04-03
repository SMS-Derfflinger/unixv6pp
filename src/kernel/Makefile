include-path := $(abs_srctree)/src $(abs_srctree)/src/include

objs := Kernel.o Lib.o main.o splash.o Utility.o Video.o

include $(abs_srctree)/scripts/Makefile

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-use-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH
