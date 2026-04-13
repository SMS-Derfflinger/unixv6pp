.DEFAULT_GOAL := all

NASMFLAGS := -g

boot.o: boot.asm
	nasm $(NASMFLAGS) -o $@ -f bin -d USE_VESA $<

sector2.o: sector2.S
	$(CROSS_COMPILE)as --32 -c -o $@ $<

objs := sector2.o

custom-clean:
	$(call cmd,rm,sector2.o)
	$(call cmd,rm,boot.o)

include $(abs_srctree)/scripts/Makefile
