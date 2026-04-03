.DEFAULT_GOAL = all

CROSS_COMPILE := x86_64-elf-

CC := $(CROSS_COMPILE)gcc
CXX := $(CROSS_COMPILE)g++
LD := $(CROSS_COMPILE)ld
AR := $(CROSS_COMPILE)ar

INCLUDE_PATHS := $(abs_srctree)/lib/include $(abs_srctree)/lib/src $(CURDIR)

CFLAGS := -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	  -nostdlib -fno-stack-protector -fomit-frame-pointer -fno-unwind-tables \
	  -fno-asynchronous-unwind-tables -fno-exceptions -mno-red-zone \
	  -mno-mmx -mno-sse -mno-sse2 -O0 -Wno-unused-variable \
	  $(foreach d, $(INCLUDE_PATHS), -I$(d))

CXXFLAGS := $(CFLAGS) -std=c++20
LDFLAGS := -m elf_i386 --whole-archive -Ttext-segment=0x400000 -e main1 -static
ARFLAGS := --target=elf32-i386

outs := Shell.elf.exe Shell.exe
out-libs :=
libs := $(abs_srctree)/lib/libv6pptongji.a
objs := CommandTree.o ExecuteCommand.o PreExecute.o main.o
hdrs := $(foreach d, $(INCLUDE_PATHS), $(wildcard $(d)/*.h) $(wildcard $(d)/*.hpp))

out-libnames := $(addprefix lib, $(addsuffix .a, $(out-libs)))

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

Shell.elf.exe: $(libs) $(objs) $(hdrs)
	$(LD) $(LDFLAGS) -m elf_i386 -o $@ $(libs) $(objs)

Shell.exe: $(libs) $(objs) $(hdrs)
	$(LD) $(LDFLAGS) -m i386pe -o $@ $(libs) $(objs)

define LIB_TARGET
$1: $(libs) $(objs) $(hdrs)
	$(AR) $(ARFLAGS) rcs $1 $(objs)
endef

$(foreach l, $(out-libnames), $(eval $(call LIB_TARGET, $(l))))

.PHONY: clean
clean:
	-rm -f $(objs)
	-rm -f $(outs) $(out-libnames)

.PHONY: all
all: $(out-libnames) $(outs)
