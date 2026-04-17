use eonix_sync_base::LazyLock;

use core::arch::asm;

use crate::{
    constants::PosixError,
    sync::SuperCell,
    tty::{console_tty, sleep_on_input_channel},
    user::Userspace,
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
        unsafe {
            char_device_fuck_tty();
        }
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

fn set_char_error(err: PosixError) {
    Userspace::get().set_error(err);
}

fn char_device_result(dev: i16, f: impl FnOnce(&dyn CharDevice) -> Result<(), PosixError>) {
    let result = match char_device_for_dev(dev) {
        Some(device) => f(device),
        None => Err(PosixError::ENXIO),
    };

    if let Err(err) = result {
        set_char_error(err);
    }
}

#[no_mangle]
pub extern "C" fn char_device_open(dev: i16, mode: i32) {
    char_device_result(dev, |device| device.open(dev, mode));
}

#[no_mangle]
pub extern "C" fn char_device_close(dev: i16, mode: i32) {
    char_device_result(dev, |device| device.close(dev, mode));
}

#[no_mangle]
pub extern "C" fn char_device_read(dev: i16) {
    let ioparam = Userspace::get().io_param_mut();
    if ioparam.m_count == 0 {
        return;
    }

    let out = ioparam.m_base as *mut u8;
    if out.is_null() {
        set_char_error(PosixError::EFAULT);
        return;
    }

    let out = unsafe { core::slice::from_raw_parts_mut(out, ioparam.m_count) };
    let result = match char_device_for_dev(dev) {
        Some(device) => device.read(dev, out),
        None => Err(PosixError::ENXIO),
    };

    match result {
        Ok(nread) => {
            ioparam.m_base += nread;
            ioparam.m_count -= nread;
        }
        Err(err) => set_char_error(err),
    }
}

#[no_mangle]
pub extern "C" fn char_device_write(dev: i16) {
    let ioparam = Userspace::get().io_param_mut();
    if ioparam.m_count == 0 {
        return;
    }

    let data = ioparam.m_base as *const u8;
    if data.is_null() {
        set_char_error(PosixError::EFAULT);
        return;
    }

    let data = unsafe { core::slice::from_raw_parts(data, ioparam.m_count) };
    let result = match char_device_for_dev(dev) {
        Some(device) => device.write(dev, data),
        None => Err(PosixError::ENXIO),
    };

    match result {
        Ok(nwritten) => {
            ioparam.m_base += nwritten;
            ioparam.m_count -= nwritten;
        }
        Err(err) => set_char_error(err),
    }
}

unsafe extern "C" {
    fn char_device_fuck_tty();
}
