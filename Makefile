.DEFAULT_GOAL := all

this-makefile := $(lastword $(MAKEFILE_LIST))
export abs_srctree := $(realpath $(dir $(this-makefile)))
export abs_objtree := $(CURDIR)

target-dirs := lib shell programs src
build-dirs := $(addprefix build-,$(target-dirs))
clean-dirs := $(addprefix clean-,$(target-dirs))

define BUILD_DIR
.PHONY: $(addprefix build-, $1)
$(addprefix build-,$1):
	make -C $1

.PHONY: $(addprefix clean-, $1)
$(addprefix clean-, $1):
	make -C $1 clean
endef

$(foreach d, $(target-dirs), $(eval $(call BUILD_DIR, $(d))))

.PHONY: install-buildrequires
install-buildrequires:
	brew install gmake x86_64-elf-gcc nasm

.PHONY: clean
clean: $(clean-dirs)

.PHONY: all
all: $(build-dirs)
