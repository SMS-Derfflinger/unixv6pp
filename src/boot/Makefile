.DEFAULT_GOAL := all

boot.o: boot.asm
	nasm -o $@ -f bin -d USE_VESA $<

sector2.o: sector2.asm
	nasm -o $@ -f elf32 $<

.PHONY: clean
clean:
	-rm sector2.o boot.o

.PHONY: all
all: boot.o sector2.o
