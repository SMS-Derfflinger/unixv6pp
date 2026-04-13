use core::arch::asm;

use bitflags::bitflags;

use crate::{println, println_info};
use crate::constants::PosixError;

#[derive(Clone, Copy)]
pub struct Port8 {
    no: u16,
}

impl Port8 {
    pub const fn new(no: u16) -> Self {
        Self { no }
    }

    pub fn read(&self) -> u8 {
        let data;
        unsafe {
            asm!(
                "inb %dx, %al",
                in("dx") self.no,
                out("al") data,
                options(att_syntax, nomem, nostack)
            )
        };

        data
    }

    pub fn write(&self, data: u8) {
        unsafe {
            asm!(
                "outb %al, %dx",
                in("al") data,
                in("dx") self.no,
                options(att_syntax, nomem, nostack)
            )
        };
    }
}

pub struct SerialIO {
    tx_rx: Port8,
    int_ena: Port8,
    int_ident: Port8,
    line_control: Port8,
    modem_control: Port8,
    line_status: Port8,
    modem_status: Port8,
    scratch: Port8,
}

impl SerialIO {
    /// Creates a new `SerialIO` instance with the given physical address.
    ///
    /// # Safety
    /// This function is unsafe because it assumes that the provided `base` is a valid IO port
    /// base for the serial port. The caller must ensure that this port base is correct.
    pub unsafe fn new(base: u16) -> Self {
        Self {
            tx_rx: Port8::new(base),
            int_ena: Port8::new(base + 1),
            int_ident: Port8::new(base + 2),
            line_control: Port8::new(base + 3),
            modem_control: Port8::new(base + 4),
            line_status: Port8::new(base + 5),
            modem_status: Port8::new(base + 6),
            scratch: Port8::new(base + 7),
        }
    }

    pub fn tx_rx(&self) -> impl SerialRegister {
        self.tx_rx
    }

    pub fn int_ena(&self) -> impl SerialRegister {
        self.int_ena
    }

    pub fn int_ident(&self) -> impl SerialRegister {
        self.int_ident
    }

    pub fn line_control(&self) -> impl SerialRegister {
        self.line_control
    }

    pub fn modem_control(&self) -> impl SerialRegister {
        self.modem_control
    }

    pub fn line_status(&self) -> impl SerialRegister {
        self.line_status
    }

    #[allow(unused)]
    pub fn modem_status(&self) -> impl SerialRegister {
        self.modem_status
    }

    #[allow(unused)]
    pub fn scratch(&self) -> impl SerialRegister {
        self.scratch
    }
}

bitflags! {
    struct LineStatus: u8 {
        const RX_READY = 0x01;
        const TX_READY = 0x20;
    }
}

trait SerialRegister {
    fn read(&self) -> u8;
    fn write(&self, value: u8);
}

impl SerialRegister for Port8 {
    fn read(&self) -> u8 {
        self.read()
    }

    fn write(&self, data: u8) {
        self.write(data);
    }
}

#[allow(dead_code)]
struct Serial {
    id: u32,
    ioregs: SerialIO,
}

impl Serial {
    fn line_status(&self) -> LineStatus {
        LineStatus::from_bits_truncate(self.ioregs.line_status().read())
    }

    pub fn new(id: u32, ioregs: SerialIO) -> KResult<Self> {
        ioregs.int_ena().write(0x00); // Disable all interrupts
        ioregs.line_control().write(0x80); // Enable DLAB (set baud rate divisor)
        ioregs.tx_rx().write(0x00); // Set divisor to 0 (lo byte) 115200 baud rate
        ioregs.int_ena().write(0x00); //              0 (hi byte)
        ioregs.line_control().write(0x03); // 8 bits, no parity, one stop bit
        ioregs.int_ident().write(0xc7); // Enable FIFO, clear them, with 14-byte threshold
        ioregs.modem_control().write(0x0b); // IRQs enabled, RTS/DSR set
        ioregs.modem_control().write(0x1e); // Set in loopback mode, test the serial chip
        ioregs.tx_rx().write(0x19); // Test serial chip (send byte 0x19 and check if serial returns
                                    // same byte)
        if ioregs.tx_rx().read() != 0x19 {
            return Err(PosixError::EIO);
        }

        ioregs.modem_control().write(0x0f); // Return to normal operation mode

        Ok(Self { id, ioregs })
    }

    fn write(&self, ch: u8) {
        self.ioregs.tx_rx().write(ch);
    }
}

macro_rules! define_static {
    {} => {};
    {lazy static $name:ident: $type:ty;} => {
        static mut $name: core::mem::MaybeUninit<$type> = core::mem::MaybeUninit::zeroed();
    };
}

define_static! {
    lazy static SERIAL: Serial;
}

pub type KResult<T> = Result<T, PosixError>;

fn init() -> KResult<()> {
    let (com0, com1) = unsafe {
        const COM0_BASE: u16 = 0x3f8;
        const COM1_BASE: u16 = 0x2f8;
        // SAFETY: The COM ports are well-known hardware addresses.
        (SerialIO::new(COM0_BASE), SerialIO::new(COM1_BASE))
    };

    let serial = Serial::new(0, com0).unwrap();
    unsafe {
        SERIAL.write(serial);
    }

    Ok(())
}

pub fn serial_write_bytes(byte_iter: impl Iterator<Item = u8>) {
    let serial = unsafe { SERIAL.assume_init_ref() };

    for ch in byte_iter {
        serial.write(ch);
    }
}

pub fn serial_write(string: &str) {
    serial_write_bytes(string.bytes());
}

macro_rules! exported {
    {$(pub fn $name:ident($($arg_name:ident: $arg_type:ty),*) $(-> $ret_type:ty)? $body:block)*} => {
        $(
        #[no_mangle]
        pub extern "C" fn $name($($arg_name: $arg_type),*) $(-> $ret_type)? $body
        )*
    };
}

exported! {
    pub fn init_serial() -> i32 {
        init().unwrap();

        println!();
        println_info!("Serial initialized");

        0
    }

    pub fn serial_write_cstr(cstr: *const u8) {
        let mut cstr = cstr;

        if cstr.is_null() {
            panic!("Null pointer");
        }

        let iter = core::iter::from_fn(move || unsafe {
            let ch = *cstr;
            if ch == 0 {
                return None;
            }

            cstr = cstr.add(1);
            Some(ch)
        });

        serial_write_bytes(iter)
    }
}
