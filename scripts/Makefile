CROSS_COMPILE :=

CC = $(CROSS_COMPILE)gcc
CXX = $(CROSS_COMPILE)g++
LD = $(CROSS_COMPILE)ld
AR = $(CROSS_COMPILE)ar

include-path :=

CFLAGS = -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	 -nostdlib -fno-stack-protector -fomit-frame-pointer -fno-unwind-tables \
	 -fno-asynchronous-unwind-tables -fno-exceptions -mno-red-zone \
	 -mno-mmx -mno-sse -mno-sse2 -O0 -Wno-unused-variable \
	 $(foreach d, $(include-path), -I$(d))

CXXFLAGS = $(CFLAGS) -std=c++20
LDFLAGS =
ARFLAGS = --target=elf32-i386

out-bins :=
out-libs :=

define BUILD_BIN
$$($$(addsuffix -hdrs, $1)) += $$(foreach d, $$(include-path), \
	$$(wildcard $$(d)/*.h) $$(wildcard $$(d)/*.hpp))

$1: $$($$(addsuffix -objs, $1)) $$($$(addsuffix -hdrs, $1)) $$($$(addsuffix -libs, $1))
	$(LD) $(LDFLAGS) $$($$(addsuffix -LDFLAGS, $1)) -o $$@ \
		$$($$(addsuffix -libs, $1)) $$($$(addsuffix -objs, $1))
endef

out-libnames := $(addprefix lib, $(addsuffix .a, $(out-libs)))

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

.PHONY: clean
clean:
	-rm -f $(objs)
	-rm -f $(out-bins) $(out-libnames)

.PHONY: all
all: $(out-libnames) $(out-bins)

.DEFAULT_GOAL = all
