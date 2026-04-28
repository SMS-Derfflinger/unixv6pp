include-path := $(abs_srctree)/lib/include $(abs_srctree)/lib/src

sources := $(wildcard *.c)
names := $(basename $(sources))
out-bins := $(addsuffix .exe, $(names))

$(foreach bin, $(out-bins), $(eval \
	$(bin)-LDFLAGS += --whole-archive -Ttext-segment=0x400000 \
	-e _start -static))

$(foreach name, $(names), $(eval $(name).exe-objs := $(name).o))
$(foreach name, $(names), $(eval $(name).exe-hdrs :=))
$(foreach name, $(names), $(eval $(name).exe-libs := $(abs_srctree)/lib/libv6pptongji.a))

include $(abs_srctree)/scripts/Makefile

CROSS_COMPILE ?= riscv64-unknown-elf-
CFLAGS := -Wall -g -march=rv64gc -mabi=lp64d -ffreestanding -fno-builtin \
	  -fno-pic -fno-pie -nostdinc -nostdlib -fno-stack-protector \
	  -fno-unwind-tables -fno-asynchronous-unwind-tables -O0 \
	  -Wno-unused-variable $(CPPFLAGS)
LDFLAGS := -m elf64lriscv

cmd_install = cp $(2).exe $(INSTALL_PATH)/$(2)
quiet_cmd_install = "INSTALL	$(2)"

cmd_check = [ -d $(2) ]
quiet_cmd_check = "CHECK	$(2)"

.PHONY: %.install
%.install: %.exe
	$(call cmd,install,$(patsubst %.exe,%,$<))

.PHONY: check-path
check-path:
	$(call cmd,check,$(INSTALL_PATH))

.PHONY: install
install: all check-path $(addsuffix .install,$(names))
