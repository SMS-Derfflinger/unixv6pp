#![no_std]

#[cfg(target_arch = "x86")]
extern crate alloc;

#[cfg(target_arch = "x86")]
pub mod compat;
#[cfg(target_arch = "x86")]
mod constants;
#[cfg(target_arch = "x86")]
mod dev;
#[cfg(target_arch = "x86")]
mod fs;
#[cfg(target_arch = "x86")]
mod interrupt;
#[cfg(target_arch = "x86")]
mod kernel;
#[cfg(target_arch = "x86")]
mod loader;
#[cfg(target_arch = "x86")]
pub mod machine;
#[cfg(target_arch = "x86")]
pub mod mm;
#[cfg(target_arch = "x86")]
mod print;
#[cfg(target_arch = "x86")]
pub mod proc;
#[cfg(target_arch = "x86")]
mod serial;
#[cfg(target_arch = "x86")]
pub mod sync;
#[cfg(target_arch = "x86")]
pub mod tty;
#[cfg(target_arch = "x86")]
mod user;
#[cfg(target_arch = "x86")]
mod vesa;

use core::panic::PanicInfo;
#[cfg(target_arch = "riscv64")]
use core::arch::naked_asm;

pub trait Ext {
    fn as_buffer(&mut self) -> &mut [u8];
}

impl<T> Ext for T where T: Copy {
    fn as_buffer(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

impl<T> Ext for [T] where T: Copy {
    fn as_buffer(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                self.len() * core::mem::size_of::<T>(),
            )
        }
    }
}

#[cfg(target_arch = "riscv64")]
#[unsafe(link_section = ".bss.stack")]
static BOOT_STACK: [u8; 4096 * 4] = [0; 4096 * 4];

#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text._start")]
unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "la   sp, {boot_stack}",
        "li   t0, {stack_size}",
        "add  sp, sp, t0",
        "la   t0, {rust_entry}",
        "jr   t0",
        boot_stack = sym BOOT_STACK,
        stack_size = const BOOT_STACK.len(),
        rust_entry = sym riscv64_rust_entry,
    )
}

#[cfg(target_arch = "riscv64")]
#[unsafe(no_mangle)]
extern "C" fn riscv64_rust_entry(hart_id: usize, dtb_addr: usize) -> ! {
    clear_bss();
    write_str("rust_kernel: entered riscv64 rust entry via OpenSBI\n");
    write_str("  hartid = ");
    write_hex(hart_id);
    write_str("\n  dtb    = ");
    write_hex(dtb_addr);
    write_str("\n");

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(target_arch = "riscv64")]
fn clear_bss() {
    unsafe extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }

    let start = &raw mut __bss_start;
    let end = &raw mut __bss_end;
    let len = (end as usize).wrapping_sub(start as usize);

    unsafe {
        core::ptr::write_bytes(start, 0, len);
    }
}

#[cfg(target_arch = "riscv64")]
fn write_str(s: &str) {
    for byte in s.bytes() {
        sbi::legacy::console_putchar(byte);
    }
}

#[cfg(target_arch = "riscv64")]
fn write_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    write_str("0x");
    for shift in (0..16).rev() {
        let nibble = ((value >> (shift * 4)) & 0xf) as usize;
        sbi::legacy::console_putchar(HEX[nibble]);
    }
}

#[cfg(target_arch = "x86")]
#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let msg = info.message();

    if let Some(msg) = msg.as_str() {
        println_fatal!("KERNEL PANIC: {}", msg);
    } else {
        println_fatal!("KERNEL PANIC: Unknown");
    }

    if let Some(loc) = info.location() {
        println_fatal!("Panicked at {}:{}", loc.file(), loc.line());
    }

    loop {}
}

#[cfg(target_arch = "riscv64")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    write_str("rust_kernel: panic\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
