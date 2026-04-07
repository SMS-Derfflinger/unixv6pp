# CROSS_COMPILE
# include-path
# out-bins
# out-libs

CC = $(CROSS_COMPILE)gcc
CXX = $(CROSS_COMPILE)g++
LD = $(CROSS_COMPILE)ld
AR = $(CROSS_COMPILE)ar

CFLAGS = -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	 -nostdlib -fno-stack-protector -fno-unwind-tables \
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

define BUILD_LIB
$1-hdrs += $$(foreach d, $$(include-path), $$(wildcard $$(d)/*.h) $$(wildcard $$(d)/*.hpp))
$1: $$($1-objs) $$($1-hdrs) $$($1-libs)
	$$(AR) $$(ARFLAGS) $$($1-ARFLAGS) rcs $1 $$($1-objs) $$($1-libs)
endef

out-libnames := $(addprefix lib, $(addsuffix .a, $(out-libs)))

.PRECIOUS: .%.c.cmd .%.cpp.cmd

.%.c.cmd: %.c
	echo "{\"command\": \"$(CC) $(CFLAGS) -c $<\", \"directory\": \"$(realpath $(abs_srctree))\", \"file\": \"$(realpath $<)\"}" > $@

.%.cpp.cmd: %.cpp
	echo "{\"command\": \"$(CXX) $(CXXFLAGS) -c $<\", \"directory\": \"$(realpath $(abs_srctree))\", \"file\": \"$(realpath $<)\"}" > $@

%.o: %.c .%.c.cmd
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.cpp .%.cpp.cmd
	$(CXX) $(CXXFLAGS) -c -o $@ $<

hdrs += $(foreach d,$(include-path),$(wildcard $(d)/*.h) $(wildcard $(d)/*.hpp))
objs.a: $(objs) $(hdrs) $(libs)
	$(AR) $(ARFLAGS) rcs $@ $(objs) $(libs)

.PHONY: %/objs.a
%/objs.a: %/
	make -C $< objs.a

.PHONY: %/objs.a.clean
%/objs.a.clean: %/
	make -C $< clean

$(foreach bin, $(out-bins), $(eval $(call BUILD_BIN,$(bin))))
$(foreach lib, $(out-libnames), $(eval $(call BUILD_LIB,$(lib))))

.PHONY: clean custom-clean
clean: $(addsuffix .clean,$(filter %/objs.a,\
	$(foreach bin,$(out-bins),$($(bin)-libs)))) custom-clean
	-rm -f $(foreach bin,$(out-bins),$($(bin)-objs))
	-rm -f $(foreach lib,$(out-libnames),$($(lib)-objs))
	-rm -f $(out-bins)
	-rm -f $(out-libnames)
	-rm -f objs.a $(objs)
	-rm -f .*.cmd
	-rm -f collect-commands.cmd

.PHONY: all
all: $(out-libnames) $(out-bins)

.PHONY: %/collect-commands.cmd
%/collect-commands.cmd: %/
	make -C $< collect-commands.cmd

collect-commands.cmd: $(subst objs.a,collect-commands.cmd,$(filter %/objs.a, $(libs))) \
	$(subst objs.a,collect-commands.cmd,$(filter %/objs.a, $(foreach bin,$(out-bins),$($(bin)-libs))))
	sh -c "cat .*.cmd > $@ || true"
	cat $^ >> $@

.DEFAULT_GOAL = all
