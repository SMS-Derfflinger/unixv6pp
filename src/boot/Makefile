include-path := $(abs_srctree)/src $(abs_srctree)/src/include $(abs_srctree)/fs

.DEFAULT_GOAL := all

ASFLAGS += --32
CPPFLAGS += -DUSE_VESA

%.i: %
	$(call cmd,cpp)

boot.o: boot.S.i
	$(call cmd,s_to_o)

out-bins = boot.bin
boot.bin-objs = boot.o
boot.bin-LDFLAGS = -Ttext 0x7c00 -m elf_i386 -static --oformat binary

sector2.o: sector2.S
	$(call cmd,s_to_o)

objs := sector2.o

custom-clean: boot.S.i.clean

include $(abs_srctree)/scripts/Makefile
