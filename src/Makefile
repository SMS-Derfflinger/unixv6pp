abs_srctree ?= $(realpath $(CURDIR)/..)

rust-output-dir = $(abs_srctree)/target
rust-profile = release
rust-target-triplet = riscv64gc-unknown-none-elf
rust-target-dir = $(rust-output-dir)/$(rust-target-triplet)/$(rust-profile)
rust-lib = $(rust-target-dir)/librust_kernel.a

CARGOFLAGS = \
	--manifest-path Cargo.toml \
	--lib \
	--target $(rust-target-triplet) \
	-Z build-std=core,alloc,compiler_builtins \
	-Z build-std-features=compiler-builtins-mem \
	$(if $(F),--features $(F))

ifeq ($(rust-profile),release)
CARGOFLAGS += --release
endif

PHONY += rust kernel.elf check clean
.PHONY: FORCE

rust: $(rust-lib)

$(rust-lib): FORCE
	cd $(abs_srctree) && CARGO_TARGET_DIR=$(rust-output-dir) cargo +nightly-2026-04-07 rustc $(CARGOFLAGS)

kernel.elf: $(rust-lib) FORCE
	rust-lld -flavor gnu -m elf64lriscv -T $(abs_srctree)/src/kernel-riscv64.ld \
		--whole-archive $(rust-lib) --no-whole-archive -o $@

check:
	cd $(abs_srctree) && CARGO_TARGET_DIR=$(rust-output-dir) cargo +nightly-2026-04-07 check \
		--manifest-path Cargo.toml \
		--lib \
		--target $(rust-target-triplet) \
		-Z build-std=core,alloc,compiler_builtins \
		-Z build-std-features=compiler-builtins-mem

clean:
	rm -f kernel.elf
