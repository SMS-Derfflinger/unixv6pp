.DEFAULT_GOAL = all

CROSS_COMPILE := x86_64-elf-

CC := $(CROSS_COMPILE)gcc
CXX := $(CROSS_COMPILE)g++
LD := $(CROSS_COMPILE)ld
AR := $(CROSS_COMPILE)ar

INCLUDE_PATHS := $(abs_srctree)/lib/include $(abs_srctree)/lib/src

CFLAGS := -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	  -nostdlib -fno-stack-protector -fomit-frame-pointer -fno-unwind-tables \
	  -fno-asynchronous-unwind-tables -fno-exceptions -mno-red-zone \
	  -mno-mmx -mno-sse -mno-sse2 -O0 -Wno-unused-variable \
	  $(foreach d, $(INCLUDE_PATHS), -I$(d))

CXXFLAGS := $(CFLAGS) -std=c++20
LDFLAGS := -m elf_i386
ARFLAGS := --target=elf32-i386

outs :=
out-libs := v6pptongji
libs :=
objs := src/double.o src/file.o src/float.o src/malloc.o src/print_parse.o src/sprintf.o \
	src/stdio.o src/stdlib.o src/string.o src/sys.o src/time.o
hdrs := $(foreach d, $(INCLUDE_PATHS), $(wildcard $(d)/*.h) $(wildcard $(d)/*.hpp))

out-libnames := $(addprefix lib, $(addsuffix .a, $(out-libs)))

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

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
