# CROSS_COMPILE
# include-path
# out-bins
# out-libs

MAKE = make

HOSTCC = gcc
HOSTCXX = g++
HOSTLD = g++
HOSTAR = ar

CC = $(CROSS_COMPILE)gcc
CXX = $(CROSS_COMPILE)g++
AS = $(CROSS_COMPILE)as
LD = $(CROSS_COMPILE)ld
AR = $(CROSS_COMPILE)ar

NASM = nasm

CFLAGS = -Wall -g -m32 -nostartfiles -fno-builtin -fno-pic -fno-pie -nostdinc \
	 -nostdlib -fno-stack-protector -fno-unwind-tables \
	 -fno-asynchronous-unwind-tables -fno-exceptions -mno-red-zone \
	 -mno-mmx -mno-sse -mno-sse2 -O0 -Wno-unused-variable \
	 $(foreach d, $(include-path), -I$(d))

CXXFLAGS = $(CFLAGS) -std=c++20
LDFLAGS =
ARFLAGS = --target=elf32-i386

HOSTCFLAGS = -Wall $(foreach d, $(include-path), -I$(d))
HOSTCXXFLAGS = $(HOSTCFLAGS)
HOSTLDFLAGS =
HOSTARFLAGS =

cmd_ld = $(LD) $(LDFLAGS) $($(2)-LDFLAGS) -o $@ $($(2)-objs) $($(2)-libs)
quiet_cmd_ld = "LD	$@"

define BUILD_BIN
$1-hdrs += $$(foreach d, $$(include-path), $$(wildcard $$(d)/*.h) $$(wildcard $$(d)/*.hpp))
$1: $$($1-objs) $$($1-hdrs) $$($1-libs)
	$$(call cmd,ld,$1)
endef

cmd_hostcxx_to_o = $(HOSTCXX) $(HOSTCXXFLAGS) -c -o $@ $<
quiet_cmd_hostcxx_to_o = "HOSTCXX	$@"

cmd_hostar = $(HOSTAR) $(HOSTARFLAGS) rcs $@ $(filter-out $(PHONY),$^)
quiet_cmd_hostar = "HOSTAR	$@"

cmd_hostld = $(HOSTLD) $(HOSTLDFLAGS) $($(2)-HOSTLDFLAGS) -o $@ $($(2)-objs) $($(2)-libs)
quiet_cmd_hostld = "HOSTLD	$@"

define BUILD_HOST_BIN
$$($1-objs): .%.cpp.cmd: %.cpp
	echo "{\"command\": \"$(HOSTCXX) $(HOSTCXXFLAGS) -c $<\", \"directory\": \"$(realpath $(abs_srctree))\", \"file\": \"$(realpath $<)\"}" > $@

$$($1-objs): %.o: %.cpp .%.cpp.cmd
	$$(call cmd,hostcxx_to_o)

$1-hdrs += $$(foreach d, $$(include-path), $$(wildcard $$(d)/*.h) $$(wildcard $$(d)/*.hpp))
$1: $$($1-objs) $$($1-hdrs) $$($1-libs)
	$$(call cmd,hostld,$1)
endef

cmd_ar = $(AR) $(ARFLAGS) $(if $(2),$($(2)ARFLAGS)) rcs $@ $($(2)objs) $($(2)libs)
quiet_cmd_ar = "AR	$@"

define BUILD_LIB
$1-hdrs += $$(foreach d, $$(include-path), $$(wildcard $$(d)/*.h) $$(wildcard $$(d)/*.hpp))
$1: $$($1-objs) $$($1-hdrs) $$($1-libs)
	$$(call cmd,ar,$1-)
endef

out-libnames := $(addprefix lib, $(addsuffix .a, $(out-libs)))

.PRECIOUS: .%.c.cmd .%.cpp.cmd

.%.c.cmd: %.c
	@echo "{\"command\": \"$(CC) $(CFLAGS) -c $<\", \"directory\": \"$(realpath $(abs_srctree))\", \"file\": \"$(realpath $<)\"}" > $@

.%.cpp.cmd: %.cpp
	@echo "{\"command\": \"$(CXX) $(CXXFLAGS) -c $<\", \"directory\": \"$(realpath $(abs_srctree))\", \"file\": \"$(realpath $<)\"}" > $@

cmd_cc_to_o = $(CC) $(CFLAGS) -c -o $@ $<
quiet_cmd_cc_to_o = "CC	$@"

cmd_cxx_to_o = $(CXX) $(CXXFLAGS) -c -o $@ $<
quiet_cmd_cxx_to_o = "CXX	$@"

cmd_s_to_o = $(AS) $(ASFLAGS) -c -o $@ $<
quiet_cmd_s_to_o = "AS	$@"

cmd_asm_to_o = $(NASM) $(NASMFLAGS) -o $@ $<
quiet_cmd_asm_to_o = "NASM	$@"

%.o: %.c .%.c.cmd
	$(call cmd,cc_to_o)

%.o: %.cpp .%.cpp.cmd
	$(call cmd,cxx_to_o)

host-obj-cmds = $(subst .o,.cpp.cmd,$(host-objs))

$(host-obj-cmds): %.cpp.cmd: %.cpp
	@echo "{\"command\": \"$(HOSTCXX) $(HOSTCXXFLAGS) -c $<\", \"directory\": \"$(realpath $(abs_srctree))\", \"file\": \"$(realpath $<)\"}" > $@

$(host-objs): %.o: %.cpp %.cpp.cmd
	$(call cmd,hostcxx_to_o)

hdrs += $(foreach d,$(include-path),$(wildcard $(d)/*.h) $(wildcard $(d)/*.hpp))
objs.a: $(objs) $(hdrs) $(libs)
	$(call cmd,ar,)

PHONY += dep-hdrs
dep-hdrs: $(hdrs)
	@:

.PHONY: $(PHONY)

host-objs.a: $(host-objs) $(host-libs) dep-hdrs
	$(call cmd,hostar)

Q = @

quiet = $(if $(Q),quiet_,)

quiet_log_print = $(if $(quiet_cmd_$(1)), echo "	$(quiet_cmd_$(1))";)
log_print = echo "$(cmd_$(1))";

cmd = @$(if $(cmd_$(1)),set -e; $($(quiet)log_print) $(cmd_$(1)),:)

submake = $(MAKE) --no-print-directory -C $<

cmd_submake = $(submake) $(2)
quiet_cmd_submake = "MAKE	$<"

cmd_submake_clean = $(submake) clean
quiet_cmd_submake_clean = "CLEAN	$<"

cmd_rm = $(if $(2),rm -f $(2),:)
quiet_cmd_rm = $(if $(2),"REMOVE	$(2)")

.PHONY: %/objs.a
%/objs.a: %/
	$(call cmd,submake,objs.a)

.PHONY: %/host-objs.a
%/host-objs.a: %/
	$(call cmd,submake,host-objs.a)

.PHONY: %/objs.a.clean
%/objs.a.clean: %/
	$(call cmd,submake_clean)

.PHONY: %/host-objs.a.clean
%/host-objs.a.clean: %/
	$(call cmd,submake_clean)

$(foreach bin, $(out-bins), $(eval $(call BUILD_BIN,$(bin))))
$(foreach bin, $(out-host-bins), $(eval $(call BUILD_HOST_BIN,$(bin))))
$(foreach lib, $(out-libnames), $(eval $(call BUILD_LIB,$(lib))))

clean_files = $(foreach item,$(1),$(call cmd,rm,$(item)))

.PHONY: clean custom-clean
clean: $(addsuffix .clean,\
		$(filter %/objs.a, $(foreach bin,$(out-bins),$($(bin)-libs))) \
		$(filter %/host-objs.a, $(foreach bin,$(out-host-bins),$($(bin)-libs))) \
	) custom-clean
	$(call clean_files,$(foreach bin,$(out-bins),$($(bin)-objs)))
	$(call clean_files,$(foreach bin,$(out-host-bins),$($(bin)-objs)))
	$(call clean_files,$(foreach lib,$(out-libnames),$($(lib)-objs)))
	$(call clean_files,$(out-bins))
	$(call clean_files,$(out-host-bins))
	$(call clean_files,$(out-libnames))
	$(call clean_files,objs.a $(objs))
	$(call clean_files,host-objs.a $(host-objs))
	$(call clean_files,$(wildcard *.cmd .*.cmd))
	$(call clean_files,collect-commands.cmd)

.PHONY: all
all: $(out-libnames) $(out-bins) $(out-host-bins)

.PHONY: %/collect-commands.cmd
%/collect-commands.cmd: %/
	$(call cmd,submake,collect-commands.cmd)

collect-commands.cmd: $(subst objs.a,collect-commands.cmd,$(filter %/objs.a, $(libs))) \
	$(subst objs.a,collect-commands.cmd,$(filter %/objs.a, $(foreach bin,$(out-bins),$($(bin)-libs)))) \
	$(subst host-objs.a,collect-commands.cmd,$(filter %/host-objs.a, $(foreach bin,$(out-host-bins),$($(bin)-libs))))
	sh -c "cat .*.cmd **/*.cmd > $@ || true"
	- [ "$^" ] && cat $^ >> $@

.DEFAULT_GOAL = all
