.DEFAULT_GOAL := all

this-makefile := $(lastword $(MAKEFILE_LIST))
export abs_srctree := $(realpath $(dir $(this-makefile)))
export abs_objtree := $(CURDIR)

.PHONY: build-lib
build-lib:
	make -C lib

.PHONY: build-shell
build-shell:
	make -C shell

.PHONY: all
all: build-lib build-shell
