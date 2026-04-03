include-path := $(abs_srctree)/src $(abs_srctree)/src/include

out-bins := kernel.exe

kernel.exe-objs :=
kernel.exe-libs := boot/objs.a dev/objs.a elf/objs.a fs/objs.a \
		   interrupt/objs.a kernel/objs.a machine/objs.a mm/objs.a \
		   pe/objs.a proc/objs.a tty/objs.a vesa/objs.a
kernel.exe-LDFLAGS := --whole-archive -m elf_i386 -T kernel.link.ld -static

kernel.bin: kernel.exe
	$(CROSS_COMPILE)objcopy -O binary $< $@

include $(abs_srctree)/scripts/Makefile

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-use-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH
