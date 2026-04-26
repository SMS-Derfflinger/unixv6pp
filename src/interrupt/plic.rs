use core::ptr::{read_volatile, write_volatile};

use crate::constants::platform::{PLIC_BASE, UART0_IRQ};

const ENABLE_OFFSET: usize = 0x2000;
const THRESHOLD_OFFSET: usize = 0x200000;
const CLAIM_COMPLETE_OFFSET: usize = 0x200004;
const ENABLE_STRIDE: usize = 0x80;
const CONTEXT_STRIDE: usize = 0x1000;

const SUPERVISOR_CONTEXT: usize = 1;

#[inline]
fn priority_ptr(interrupt: usize) -> *mut u32 {
    (PLIC_BASE + interrupt * core::mem::size_of::<u32>()) as *mut u32
}

#[inline]
fn enable_ptr(interrupt: usize) -> *mut u32 {
    let word = interrupt / 32;
    (PLIC_BASE + ENABLE_OFFSET + SUPERVISOR_CONTEXT * ENABLE_STRIDE + word * core::mem::size_of::<u32>())
        as *mut u32
}

#[inline]
fn threshold_ptr() -> *mut u32 {
    (PLIC_BASE + THRESHOLD_OFFSET + SUPERVISOR_CONTEXT * CONTEXT_STRIDE) as *mut u32
}

#[inline]
fn claim_complete_ptr() -> *mut u32 {
    (PLIC_BASE + CLAIM_COMPLETE_OFFSET + SUPERVISOR_CONTEXT * CONTEXT_STRIDE) as *mut u32
}

pub fn init() {
    set_threshold(0);
    set_priority(UART0_IRQ, 1);
    enable_interrupt(UART0_IRQ);
}

pub fn set_threshold(threshold: u32) {
    unsafe {
        write_volatile(threshold_ptr(), threshold);
    }
}

pub fn set_priority(interrupt: usize, priority: u32) {
    unsafe {
        write_volatile(priority_ptr(interrupt), priority);
    }
}

pub fn enable_interrupt(interrupt: usize) {
    let bit = 1u32 << (interrupt % 32);
    unsafe {
        let ptr = enable_ptr(interrupt);
        let value = read_volatile(ptr);
        write_volatile(ptr, value | bit);
    }
}

pub fn claim_interrupt() -> Option<usize> {
    let interrupt = unsafe { read_volatile(claim_complete_ptr()) } as usize;
    (interrupt != 0).then_some(interrupt)
}

pub fn complete_interrupt(interrupt: usize) {
    unsafe {
        write_volatile(claim_complete_ptr(), interrupt as u32);
    }
}
