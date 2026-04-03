include-path := $(abs_srctree)/src $(abs_srctree)/src/include

hdrs := $(wildcard sys/*.h)
objs := stdio.o string.o sys/LinkedList.o

include $(abs_srctree)/scripts/Makefile

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-use-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH
