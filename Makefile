.DEFAULT_GOAL := all

QEMU = qemu-system-i386

QEMU_FLAGS := -m 64M \
	      -rtc base=localtime \
	      -d cpu_reset -D target/qemu.log \
	      -machine pc -cpu Icelake-Server

QEMU_GDB_FLAGS := -chardev socket,path=target/qemu-gdb.sock,server=on,wait=off,id=gdb0 \
		  -gdb chardev:gdb0 -S

QEMU_DISK := -boot c -drive file=target/disk.img,if=ide,index=0,media=disk,format=raw

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

workdir := target/img-workspace

target/disk.img: $(build-dirs)
	mkdir -p $(workdir)/programs/bin
	mkdir -p $(workdir)/programs/etc
	make -C src kernel.bin
	cp src/kernel.bin $(workdir)/
	rm src/kernel.bin
	make -C src/boot boot.o
	cp src/boot/boot.o $(workdir)/boot.bin
	cp programs/*.exe $(workdir)/programs/bin/
	cp shell/Shell.exe $(workdir)/programs/
	cp tools/unixv6pp_splash/v6pp_splash.bmp $(workdir)/programs/etc/
	cd $(workdir)/ && $(abs_srctree)/tools/filescanner \
		| $(abs_srctree)/tools/FsEditor/fseditor \
		$(abs_srctree)/target/disk.img c

.PHONY: qemu
qemu: target/disk.img
	$(QEMU) $(QEMU_FLAGS) $(QEMU_DISK)

.PHONY: qemug
qemug: target/disk.img
	$(QEMU) $(QEMU_FLAGS) $(QEMU_DISK) $(QEMU_GDB_FLAGS)

.PHONY: all
all: $(build-dirs) target/disk.img

custom-clean:
	-rm -rf target/
