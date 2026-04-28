include-path := $(abs_srctree)/lib/include $(abs_srctree)/lib/src $(CURDIR)

out-bins := Shell.exe

Shell.exe-libs := $(abs_srctree)/lib/libv6pptongji.a
Shell.exe-objs := CommandTree.o ExecuteCommand.o PreExecute.o main.o
Shell.exe-LDFLAGS := --whole-archive -Ttext-segment=0x400000 -e _start \
		     -static

include $(abs_srctree)/scripts/Makefile

CROSS_COMPILE ?= riscv64-unknown-elf-
CFLAGS := -Wall -g -march=rv64gc -mabi=lp64d -ffreestanding -fno-builtin \
	  -fno-pic -fno-pie -nostdinc -nostdlib -fno-stack-protector \
	  -fno-unwind-tables -fno-asynchronous-unwind-tables -O0 \
	  -Wno-unused-variable -fomit-frame-pointer $(CPPFLAGS)
LDFLAGS := -m elf64lriscv
