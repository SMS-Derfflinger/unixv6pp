.DEFAULT_GOAL := all

NASMFLAGS += -g -f bin -d USE_VESA
ASFLAGS += --32

boot.o: boot.asm
	$(call cmd,asm_to_o)

sector2.o: sector2.S
	$(call cmd,s_to_o)

objs := sector2.o

custom-clean:
	$(call cmd,rm,sector2.o)
	$(call cmd,rm,boot.o)

include $(abs_srctree)/scripts/Makefile
