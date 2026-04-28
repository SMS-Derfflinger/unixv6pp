include-path := $(abs_srctree)/lib/include $(abs_srctree)/lib/src

out-libs := v6pptongji
libv6pptongji.a-objs = src/double.o src/file.o src/float.o src/malloc.o \
		       src/print_parse.o src/sprintf.o src/stdio.o \
		       src/stdlib.o src/string.o src/sys.o src/time.o

include $(abs_srctree)/scripts/Makefile

CROSS_COMPILE ?= riscv64-unknown-elf-
CFLAGS := -Wall -g -march=rv64gc -mabi=lp64d -ffreestanding -fno-builtin \
	  -fno-pic -fno-pie -nostdinc -nostdlib -fno-stack-protector \
	  -fno-unwind-tables -fno-asynchronous-unwind-tables -O0 \
	  -Wno-unused-variable $(CPPFLAGS)
LDFLAGS := -m elf64lriscv
ARFLAGS :=
