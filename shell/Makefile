include-path := $(abs_srctree)/lib/include $(abs_srctree)/lib/src $(CURDIR)

out-bins := Shell.exe

Shell.exe-libs := $(abs_srctree)/lib/libv6pptongji.a
Shell.exe-objs := CommandTree.o ExecuteCommand.o PreExecute.o main.o
Shell.exe-LDFLAGS := --whole-archive -Ttext-segment=0x400000 -e main1 \
		     -static -m i386pe

include $(abs_srctree)/scripts/Makefile
