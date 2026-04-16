use eonix_sync_base::LazyLock;

use crate::{
    constants::PosixError,
    serial::{serial_try_read_byte, serial_write_bytes},
    sync::SuperCell,
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
        Ok(())
    }

    fn close(&self, dev: i16, _mode: i32) -> Result<(), PosixError> {
        Self::validate(dev)
    }

    fn read(&self, dev: i16, out: &mut [u8]) -> Result<usize, PosixError> {
        Self::validate(dev)?;

        let mut nread = 0;
        for slot in out {
            let Some(byte) = serial_try_read_byte() else {
                break;
            };

            *slot = byte;
            nread += 1;

            if byte == b'\n' || byte == b'\r' {
                break;
            }
        }

        Ok(nread)
    }

    fn write(&self, dev: i16, data: &[u8]) -> Result<usize, PosixError> {
        Self::validate(dev)?;
        serial_write_bytes(data.iter().copied());
        Ok(data.len())
    }
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
