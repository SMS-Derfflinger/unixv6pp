include-path := $(abs_srctree)/lib/include $(abs_srctree)/lib/src

LDFLAGS := -m elf_i386
ARFLAGS := --target=elf32-i386

out-libs := v6pptongji
libv6pptongji.a-objs = src/double.o src/file.o src/float.o src/malloc.o \
		       src/print_parse.o src/sprintf.o src/stdio.o \
		       src/stdlib.o src/string.o src/sys.o src/time.o
libv6pptongji.a-LDFLAGS = -m elf_i386

include $(abs_srctree)/scripts/Makefile
