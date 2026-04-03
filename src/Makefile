include-path := $(abs_srctree)/src $(abs_srctree)/src/include

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH

out-bins := kernel.exe

kernel.exe-objs :=
kernel.exe-libs := boot/objs.a dev/objs.a
kernel.exe-LDFLAGS := --whole-archive -m elf_i386 -T kernel.link.ld -static

kernel.bin: kernel.exe
	$(CROSS_COMPILE)objcopy -O binary $< $@

include $(abs_srctree)/scripts/Makefile
