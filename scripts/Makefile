# CROSS_COMPILE
# include-path
# out-bins
# out-libs

CC = $(CROSS_COMPILE)gcc
CXX = $(CROSS_COMPILE)g++
LD = $(CROSS_COMPILE)ld
AR = $(CROSS_COMPILE)ar

CFLAGS = -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	 -nostdlib -fno-stack-protector -fomit-frame-pointer -fno-unwind-tables \
	 -fno-asynchronous-unwind-tables -fno-exceptions -mno-red-zone \
	 -mno-mmx -mno-sse -mno-sse2 -O0 -Wno-unused-variable \
	 $(foreach d, $(include-path), -I$(d))

CXXFLAGS = $(CFLAGS) -std=c++20
LDFLAGS =
ARFLAGS = --target=elf32-i386

define BUILD_BIN
$1-hdrs += $$(foreach d, $$(include-path), $$(wildcard $$(d)/*.h) $$(wildcard $$(d)/*.hpp))
$1: $$($1-objs) $$($1-hdrs) $$($1-libs)
	$$(LD) $$(LDFLAGS) $$($1-LDFLAGS) -o $$@ $$($1-objs) $$($1-libs)
endef

out-libnames := $(addprefix lib, $(addsuffix .a, $(out-libs)))

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

$(foreach bin, $(out-bins), $(eval $(call BUILD_BIN,$(bin))))

.PHONY: clean
clean:
	-rm -f $(foreach bin, $(out-bins), $($(bin)-objs))
	-rm -f $(out-bins) $(out-libnames)

.PHONY: all
all: $(out-libnames) $(out-bins)

.DEFAULT_GOAL = all
