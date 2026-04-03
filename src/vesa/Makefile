include-path := $(abs_srctree)/src $(abs_srctree)/src/include

objs := console.o fonts.o svga.o

include $(abs_srctree)/scripts/Makefile

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-use-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH
