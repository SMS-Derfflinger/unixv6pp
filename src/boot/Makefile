.DEFAULT_GOAL := all

NASMFLAGS := -g

boot.o: boot.asm
	nasm $(NASMFLAGS) -o $@ -f bin -d USE_VESA $<

sector2.o: sector2.asm
	nasm $(NASMFLAGS) -o $@ -f elf32 $<

objs := sector2.o

custom-clean:
	-rm sector2.o boot.o

include $(abs_srctree)/scripts/Makefile
