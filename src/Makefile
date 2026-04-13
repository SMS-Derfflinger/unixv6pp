include-path := $(abs_srctree)/src $(abs_srctree)/src/include $(abs_srctree)/fs

out-bins := kernel.exe

kernel.exe-objs :=
kernel.exe-hdrs := kernel.link.ld
kernel.exe-libs := boot/objs.a dev/objs.a elf/objs.a fs/objs.a \
		   interrupt/objs.a kernel/objs.a machine/objs.a mm/objs.a \
		   pe/objs.a proc/objs.a tty/objs.a vesa/objs.a \
		   libyrosstd/objs.a librust_kernel.a
kernel.exe-LDFLAGS := --whole-archive -m elf_i386 -T kernel.link.ld -static

.PHONY: rustlib
rustlib:
	cd $(abs_srctree) && CARGO_TARGET_DIR=target cargo \
		+nightly rustc \
		--manifest-path Cargo.toml \
		--target i686-none.json \
		-Z build-std=core,compiler_builtins,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Zjson-target-spec \
		--release

custom-clean:
	-rm libkernelrust.a

librust_kernel.a: rustlib
	ln -sf $(abs_srctree)/target/i686-none/release/librust_kernel.a $(CURDIR)/

kernel.bin: kernel.exe
	$(CROSS_COMPILE)objcopy -O binary $< $@

include $(abs_srctree)/scripts/Makefile

CXXFLAGS += -no-pie -fno-rtti -fno-threadsafe-statics -fno-use-cxa-atexit \
	    -DUSE_VESA -DENABLE_SPLASH
