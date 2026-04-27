#![no_std]

extern crate alloc;

pub mod compat;
#[cfg(target_arch = "x86")]
mod dev;
#[cfg(target_arch = "x86")]
mod fs;
mod interrupt;
#[cfg(target_arch = "x86")]
mod kernel;
#[cfg(target_arch = "x86")]
mod loader;
pub mod machine;
pub mod mm;
pub mod proc;
#[cfg(target_arch = "x86")]
pub mod tty;
mod user;
#[cfg(target_arch = "x86")]
mod vesa;

mod constants;
mod print;
mod serial;
pub mod sync;

use core::arch::naked_asm;
use core::panic::PanicInfo;

pub trait Ext {
    fn as_buffer(&mut self) -> &mut [u8];
}

impl<T> Ext for T
where
    T: Copy,
{
    fn as_buffer(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

impl<T> Ext for [T]
where
    T: Copy,
{
    fn as_buffer(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                self.len() * core::mem::size_of::<T>(),
            )
        }
    }
}

#[unsafe(link_section = ".bootstrap.stack")]
static BOOT_STACK: [u8; 4096 * 4] = [0; 4096 * 4];

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".bootstrap.entry")]
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

#[unsafe(no_mangle)]
extern "C" fn riscv64_rust_entry(hart_id: usize, dtb_addr: usize) -> ! {
    clear_bss();
    serial::init_serial();
    interrupt::init_trap();
    interrupt::init_interrupt_controller();
    println_info!("rust_kernel: entered riscv64 rust entry via OpenSBI");
    println_info!("  hartid = {:#x}", hart_id);
    println_info!("  dtb    = {:#x}", dtb_addr);
    println_info!("timer armed at {} Hz", interrupt::time::INTERRUPTS_PER_SECOND);

    #[cfg(feature = "rvdebug")]
    interrupt_test();

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(feature = "rvdebug")]
fn interrupt_test() {
    println_info!("before ebreak");
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack));
    }
    println_info!("after ebreak");
    println_info!("before illegal instruction");
    unsafe {
        core::arch::asm!(".word 0xffffffff", options(nomem, nostack));
    }
    println_info!("after illegal instruction");
}

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

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    println_fatal!("rust_kernel: panic");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
