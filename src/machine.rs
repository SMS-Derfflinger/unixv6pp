use core::arch::asm;

use crate::dev::io_port::IOPort;

#[repr(C)]
pub struct SystemTime {
    second: i32,
    minute: i32,
    hour: i32,
    day_of_month: i32,
    month: i32,
    year: i32,
    day_of_week: i32,
}

const PIT_INPUT_FREQ: i32 = 1_193_180;
const PIT_CNT0_PORT: u16 = 0x40;
const PIT_CTRLWRD_PORT: u16 = 0x43;
const PIT_CTRLCMD_SEL0: u8 = 0x00;
const PIT_CTRLCMD_MODE3: u8 = 0x06;
const PIT_CTRLCMD_RW: u8 = 0x30;

const PIC_MASTER_IO_PORT_1: u16 = 0x20;
const PIC_MASTER_IO_PORT_2: u16 = 0x21;
const PIC_SLAVE_IO_PORT_1: u16 = 0xa0;
const PIC_SLAVE_IO_PORT_2: u16 = 0xa1;
const PIC_MASTER_IRQ_START: u8 = 0x20;
const PIC_SLAVE_IRQ_START: u8 = PIC_MASTER_IRQ_START + 8;
const PIC_IRQ_SLAVE: u32 = 2;
const PIC_MASK_ALL: u8 = 0xff;

const CMOS_ADDR_PORT: u16 = 0x70;
const CMOS_DATA_PORT: u16 = 0x71;
const RTC_SECONDS: u8 = 0x00;
const RTC_MINUTES: u8 = 0x02;
const RTC_HOURS: u8 = 0x04;
const RTC_DAY_OF_WEEK: u8 = 0x06;
const RTC_DAY_OF_MONTH: u8 = 0x07;
const RTC_MONTH: u8 = 0x08;
const RTC_YEAR: u8 = 0x09;
const RTC_STATUS_REGISTER_A: u8 = 0x0a;
const RTC_UPDATE_IN_PROGRESS: i32 = 0x80;

const EXTENDED_MEMORY_ABOVE_1MB_LOW: u8 = 0x30;	/* 1MB以上扩展内存(低字节) */
const EXTENDED_MEMORY_ABOVE_1MB_HIGH: u8 = 0x31;

#[no_mangle]
pub extern "C" fn _chip8253_init(mut ticks: i32) {
    if ticks <= 0 {
        ticks = 60;
    }

    let counter = PIT_INPUT_FREQ / ticks;

    unsafe {
        IOPort::out_byte(
            PIT_CTRLWRD_PORT,
            PIT_CTRLCMD_SEL0 | PIT_CTRLCMD_MODE3 | PIT_CTRLCMD_RW,
        );
        IOPort::out_byte(PIT_CNT0_PORT, (counter % 256) as u8);
        IOPort::out_byte(PIT_CNT0_PORT, (counter / 256) as u8);
    }
}

#[no_mangle]
pub extern "C" fn _chip8259a_init() {
    unsafe {
        IOPort::out_byte(PIC_MASTER_IO_PORT_1, 0x11);
        IOPort::out_byte(PIC_SLAVE_IO_PORT_1, 0x11);

        IOPort::out_byte(PIC_MASTER_IO_PORT_2, PIC_MASTER_IRQ_START);
        IOPort::out_byte(PIC_SLAVE_IO_PORT_2, PIC_SLAVE_IRQ_START);

        IOPort::out_byte(PIC_MASTER_IO_PORT_2, (1 << PIC_IRQ_SLAVE) as u8);
        IOPort::out_byte(PIC_SLAVE_IO_PORT_2, PIC_IRQ_SLAVE as u8);

        IOPort::out_byte(PIC_MASTER_IO_PORT_2, 0x01);
        IOPort::out_byte(PIC_SLAVE_IO_PORT_2, 0x01);

        IOPort::out_byte(PIC_MASTER_IO_PORT_2, PIC_MASK_ALL);
        IOPort::out_byte(PIC_SLAVE_IO_PORT_2, PIC_MASK_ALL);
    }
}

#[no_mangle]
pub extern "C" fn _chip8259a_irq_enable(irq: u32) {
    if irq >= 16 {
        return;
    }

    unsafe {
        if irq <= 7 {
            let value = IOPort::in_byte(PIC_MASTER_IO_PORT_2) & !(1 << irq);
            IOPort::out_byte(PIC_MASTER_IO_PORT_2, value);
        } else {
            let value = IOPort::in_byte(PIC_SLAVE_IO_PORT_2) & !(1 << (irq - 8));
            IOPort::out_byte(PIC_SLAVE_IO_PORT_2, value);
        }
    }
}

#[no_mangle]
pub extern "C" fn _chip8259a_irq_disable(irq: u32) {
    if irq >= 16 {
        return;
    }

    unsafe {
        if irq <= 7 {
            let value = IOPort::in_byte(PIC_MASTER_IO_PORT_2) | (1 << irq);
            IOPort::out_byte(PIC_MASTER_IO_PORT_2, value);
        } else {
            let value = IOPort::in_byte(PIC_SLAVE_IO_PORT_2) | (1 << (irq - 8));
            IOPort::out_byte(PIC_SLAVE_IO_PORT_2, value);
        }
    }
}

#[no_mangle]
pub extern "C" fn _cmos_read_time(time: *mut SystemTime) {
    if time.is_null() {
        return;
    }

    while cmos_read_byte(RTC_STATUS_REGISTER_A) & RTC_UPDATE_IN_PROGRESS != 0 {}

    unsafe {
        (*time).second = bcd_to_binary(cmos_read_byte(RTC_SECONDS));
        (*time).minute = bcd_to_binary(cmos_read_byte(RTC_MINUTES));
        (*time).hour = bcd_to_binary(cmos_read_byte(RTC_HOURS));
        (*time).day_of_month = bcd_to_binary(cmos_read_byte(RTC_DAY_OF_MONTH));
        (*time).month = bcd_to_binary(cmos_read_byte(RTC_MONTH));
        (*time).year = bcd_to_binary(cmos_read_byte(RTC_YEAR));
        (*time).day_of_week = bcd_to_binary(cmos_read_byte(RTC_DAY_OF_WEEK));
    }
}

fn cmos_read_byte(cmos_offset: u8) -> i32 {
    unsafe {
        disable_interrupts();
        IOPort::out_byte(CMOS_ADDR_PORT, cmos_offset);
        let value = IOPort::in_byte(CMOS_DATA_PORT) as i32;
        enable_interrupts();
        value
    }
}

#[no_mangle]
pub extern "C" fn _cmos_read_byte_low() -> i32 {
    cmos_read_byte(EXTENDED_MEMORY_ABOVE_1MB_LOW)
}

#[no_mangle]
pub extern "C" fn _cmos_read_byte_high() -> i32 {
    cmos_read_byte(EXTENDED_MEMORY_ABOVE_1MB_HIGH)
}

fn bcd_to_binary(value: i32) -> i32 {
    (value & 0x0f) + ((value >> 4) * 10)
}

unsafe fn disable_interrupts() {
    asm!("cli", options(nomem, nostack));
}

unsafe fn enable_interrupts() {
    asm!("sti", options(nomem, nostack));
}
