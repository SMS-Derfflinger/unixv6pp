#![no_std]

extern crate alloc;

mod constants;
mod fs;
mod dev;
mod mm;
mod print;
pub mod proc;
mod serial;
mod tty;
mod vesa;
pub mod sync;
mod user;

use core::panic::PanicInfo;

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
