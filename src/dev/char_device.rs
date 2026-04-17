use eonix_sync_base::LazyLock;

use core::arch::asm;

use crate::{
    constants::PosixError,
    sync::SuperCell,
    tty::{console_tty, sleep_on_input_channel},
};

use super::device_manager::minor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConsoleState {
    is_open: bool,
}

impl ConsoleState {
    const fn new() -> Self {
        Self { is_open: false }
    }
}

pub trait CharDevice: Sync {
    fn open(&self, dev: i16, mode: i32) -> Result<(), PosixError>;
    fn close(&self, dev: i16, mode: i32) -> Result<(), PosixError>;
    fn read(&self, dev: i16, out: &mut [u8]) -> Result<usize, PosixError>;
    fn write(&self, dev: i16, data: &[u8]) -> Result<usize, PosixError>;
}

pub struct ConsoleDevice {
    state: SuperCell<ConsoleState>,
}

impl ConsoleDevice {
    pub const fn new() -> Self {
        Self {
            state: SuperCell::new(ConsoleState::new()),
        }
    }

    fn validate(dev: i16) -> Result<(), PosixError> {
        if minor(dev) == 0 {
            Ok(())
        } else {
            Err(PosixError::ENXIO)
        }
    }
}

impl CharDevice for ConsoleDevice {
    fn open(&self, dev: i16, _mode: i32) -> Result<(), PosixError> {
        Self::validate(dev)?;
        self.state.with_mut(|state| {
            state.is_open = true;
        });
        console_tty().with_mut(|tty| tty.open());
        Ok(())
    }

    fn close(&self, dev: i16, _mode: i32) -> Result<(), PosixError> {
        Self::validate(dev)
    }

    fn read(&self, dev: i16, out: &mut [u8]) -> Result<usize, PosixError> {
        Self::validate(dev)?;
        loop {
            unsafe {
                disable_interrupts();
            }

            if let Some(nread) = console_tty().with_mut(|tty| tty.read_available(out)) {
                unsafe {
                    enable_interrupts();
                }
                return Ok(nread);
            }

            let chan = console_tty().with(|tty| tty.read_wait_channel());
            sleep_on_input_channel(chan);
            unsafe {
                enable_interrupts();
            }
        }
    }

    fn write(&self, dev: i16, data: &[u8]) -> Result<usize, PosixError> {
        Self::validate(dev)?;
        Ok(console_tty().with_mut(|tty| tty.write(data)))
    }
}

unsafe fn disable_interrupts() {
    asm!("cli", options(nomem, nostack));
}

unsafe fn enable_interrupts() {
    asm!("sti", options(nomem, nostack));
}

static CONSOLE_DEVICE: LazyLock<ConsoleDevice> = LazyLock::new(ConsoleDevice::new);

pub fn console_device() -> &'static ConsoleDevice {
    &CONSOLE_DEVICE
}

pub fn char_device_for_major(major: i16) -> Option<&'static dyn CharDevice> {
    match major {
        0 => Some(console_device()),
        _ => None,
    }
}

pub fn char_device_for_dev(dev: i16) -> Option<&'static dyn CharDevice> {
    char_device_for_major(super::device_manager::major(dev))
}

fn errno(err: PosixError) -> i32 {
    -(err as i32)
}

#[no_mangle]
pub extern "C" fn char_device_open(dev: i16, mode: i32) -> i32 {
    match char_device_for_dev(dev) {
        Some(device) => device.open(dev, mode).map_or_else(errno, |_| 0),
        None => errno(PosixError::ENXIO),
    }
}

#[no_mangle]
pub extern "C" fn char_device_close(dev: i16, mode: i32) -> i32 {
    match char_device_for_dev(dev) {
        Some(device) => device.close(dev, mode).map_or_else(errno, |_| 0),
        None => errno(PosixError::ENXIO),
    }
}

#[no_mangle]
pub unsafe extern "C" fn char_device_read(dev: i16, out: *mut u8, count: i32) -> i32 {
    if count <= 0 {
        return 0;
    }

    let Some(out) = out.as_mut() else {
        return errno(PosixError::EFAULT);
    };

    let out = core::slice::from_raw_parts_mut(out, count as usize);
    match char_device_for_dev(dev) {
        Some(device) => device.read(dev, out).map_or_else(errno, |n| n as i32),
        None => errno(PosixError::ENXIO),
    }
}

#[no_mangle]
pub unsafe extern "C" fn char_device_write(dev: i16, data: *const u8, count: i32) -> i32 {
    if count <= 0 {
        return 0;
    }

    let Some(data) = data.as_ref() else {
        return errno(PosixError::EFAULT);
    };

    let data = core::slice::from_raw_parts(data, count as usize);
    match char_device_for_dev(dev) {
        Some(device) => device.write(dev, data).map_or_else(errno, |n| n as i32),
        None => errno(PosixError::ENXIO),
    }
}
