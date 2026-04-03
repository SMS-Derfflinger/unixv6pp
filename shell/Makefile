.DEFAULT_TARGET = all

CXX := clang++
LINKER := ld

INCLUDE_PATHS := $(abs_srctree/lib/include) $(abs_srctree/lib/src)

CXXFLAGS := -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	     -nostdlib -fno-stack-protector -fomit-frame-pointer -fno-unwind-tables \
	     -fno-asynchronous-unwind-tables -fno-exceptions -mno-red-zone \
	     -mno-mmx -mno-sse -mno-sse2 -O0 -Wno-unused-variable \
	     $(foreach d, $(INCLUDE_PATHS), -I\"$(d)\")

LDFLAGS := -m32 --whole-archive -Ttext-segment=0x400000 -e main1 -static

outs := Shell.elf.exe Shell.exe
libs := $(abs_objtree)/libv6pptongji.a
objs := CommandTree.o ExecuteCommand.o PreExecute.o main.o
hdrs := $(wildcard $(CURDIR)/*.h)

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

Shell.elf.exe: $(libs) $(objs) $(hdrs)
	$(CXX) $(LDFLAGS) -m elf_i386 -o $@ $(libs) $(objs)

Shell.exe: $(libs) $(objs) $(hdrs)
	$(CXX) $(LDFLAGS) -m i386pe -o $@ $(libs) $(objs)

.PHONY: clean
clean:
	-rm -f $(objs)
	-rm -f $(outs)

.PHONY: all
all: $(outs)
