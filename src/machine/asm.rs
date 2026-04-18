use core::arch::asm;

pub unsafe fn disable_interrupts() {
    asm!("cli", options(nomem, nostack));
}

pub unsafe fn enable_interrupts() {
    asm!("sti", options(nomem, nostack));
}
