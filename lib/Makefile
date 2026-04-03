.DEFAULT_TARGET = all

CC := x86_64-elf-gcc
CXX := x86_64-elf-g++
LD := x86_64-elf-ld

INCLUDE_PATHS := $(abs_srctree)/lib/include $(abs_srctree)/lib/src

CFLAGS := -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	  -nostdlib -fno-stack-protector -fomit-frame-pointer -fno-unwind-tables \
	  -fno-asynchronous-unwind-tables -fno-exceptions -mno-red-zone \
	  -mno-mmx -mno-sse -mno-sse2 -O0 -Wno-unused-variable \
	  $(foreach d, $(INCLUDE_PATHS), -I$(d))

CXXFLAGS := $(CFLAGS) -std=c++20
LDFLAGS := -m elf_i386

outs := libv6pptongji.a
libs :=
objs := src/double.o src/file.o src/float.o src/malloc.o src/print_parse.o src/sprintf.o \
	src/stdio.o src/stdlib.o src/string.o src/sys.o src/time.o
hdrs := $(foreach d, $(INCLUDE_PATHS), $(wildcard $(d)/*.h) $(wildcard $(d)/*.hpp))

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

libv6pptongji.a: $(libs) $(objs) $(hdrs)
	$(LD) $(LDFLAGS) -o $@ $(libs) $(objs)

.PHONY: clean
clean:
	-rm -f $(objs)
	-rm -f $(outs)

.PHONY: all
all: $(outs)


