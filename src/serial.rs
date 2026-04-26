use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::constants::platform::UART0_BASE;

const RBR_THR_DLL: usize = 0;
const IER_DLM: usize = 1;
const FCR_IIR: usize = 2;
const LCR: usize = 3;
const MCR: usize = 4;
const LSR: usize = 5;

const LCR_DLAB: u8 = 1 << 7;
const LCR_8N1: u8 = 0x03;
const FCR_ENABLE_FIFO: u8 = 0x01;
const FCR_CLEAR_FIFO: u8 = 0x06;
const MCR_DTR_RTS: u8 = 0x03;
const LSR_RX_READY: u8 = 0x01;
const LSR_TX_IDLE: u8 = 0x20;
const BAUD_DIVISOR_115200: u16 = 1;

static SERIAL_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[inline]
fn reg_ptr(offset: usize) -> *mut u8 {
    (UART0_BASE + offset) as *mut u8
}

#[inline]
fn read_reg(offset: usize) -> u8 {
    unsafe { read_volatile(reg_ptr(offset)) }
}

#[inline]
fn write_reg(offset: usize, value: u8) {
    unsafe { write_volatile(reg_ptr(offset), value) }
}

fn uart_init_once() {
    if SERIAL_INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }

    write_reg(IER_DLM, 0x00);
    write_reg(LCR, LCR_DLAB);
    write_reg(RBR_THR_DLL, (BAUD_DIVISOR_115200 & 0xff) as u8);
    write_reg(IER_DLM, (BAUD_DIVISOR_115200 >> 8) as u8);
    write_reg(LCR, LCR_8N1);
    write_reg(FCR_IIR, FCR_ENABLE_FIFO | FCR_CLEAR_FIFO);
    write_reg(MCR, MCR_DTR_RTS);
}

fn put_byte(byte: u8) {
    while read_reg(LSR) & LSR_TX_IDLE == 0 {}
    write_reg(RBR_THR_DLL, byte);
}

pub fn init_serial() {
    uart_init_once();
}

pub fn serial_write_bytes(byte_iter: impl Iterator<Item = u8>) {
    uart_init_once();

    for ch in byte_iter {
        if ch == b'\n' {
            put_byte(b'\r');
        }
        put_byte(ch);
    }
}

pub fn serial_write(string: &str) {
    serial_write_bytes(string.bytes());
}

pub fn serial_try_read_byte() -> Option<u8> {
    uart_init_once();
    (read_reg(LSR) & LSR_RX_READY != 0).then(|| read_reg(RBR_THR_DLL))
}

pub fn serial_write_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    serial_write("0x");
    for shift in (0..(core::mem::size_of::<usize>() * 2)).rev() {
        let nibble = ((value >> (shift * 4)) & 0xf) as usize;
        put_byte(HEX[nibble]);
    }
}
