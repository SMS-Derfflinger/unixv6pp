#![no_std]

mod constants;
mod mm;
mod print;
mod serial;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let msg = info.message();

    if let Some(msg) = msg.as_str() {
        println_fatal!("KERNEL PANIC: {}", msg);
    } else {
        println_fatal!("KERNEL PANIC: Unknown");
    }

    loop {}
}
