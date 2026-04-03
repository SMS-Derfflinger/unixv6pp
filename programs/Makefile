include-path := $(abs_srctree)/lib/include $(abs_srctree)/lib/src

sources := $(wildcard *.c)
names := $(basename $(sources))
out-bins := $(addsuffix .exe, $(names))

$(foreach bin, $(out-bins), $(eval \
	$(bin)-LDFLAGS += --whole-archive -Ttext-segment=0x400000 \
	-e main1 -static -m i386pe))

$(foreach name, $(names), $(eval $(name).exe-objs := $(name).o))
$(foreach name, $(names), $(eval $(name).exe-hdrs :=))
$(foreach name, $(names), $(eval $(name).exe-libs := $(abs_srctree)/lib/libv6pptongji.a))

include $(abs_srctree)/scripts/Makefile
