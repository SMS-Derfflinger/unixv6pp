use eonix_sync_base::LazyLock;

use super::{
    block_device::{block_device_for_major, BlockDevice},
    char_device::{char_device_for_major, CharDevice},
};

pub const MAX_DEVICE_NUM: usize = 10;
pub const NODEV: i16 = -1;
pub const ROOTDEV: i16 = 0;
pub const TTYDEV: i16 = 0;

pub const fn major(dev: i16) -> i16 {
    dev >> 8
}

pub const fn minor(dev: i16) -> i16 {
    dev & 0x00ff
}

pub const fn set_major(dev: i16, value: i16) -> i16 {
    (dev & 0x00ff) | (value << 8)
}

pub const fn set_minor(dev: i16, value: i16) -> i16 {
    (dev & 0xff00u16 as i16) | (value & 0x00ff)
}

pub struct DeviceManager {
    nblkdev: usize,
    nchrdev: usize,
}

impl DeviceManager {
    pub const fn new() -> Self {
        Self {
            nblkdev: 1,
            nchrdev: 1,
        }
    }

    pub fn initialize(&mut self) {
        self.nblkdev = 1;
        self.nchrdev = 1;
    }

    pub fn n_block_devices(&self) -> usize {
        self.nblkdev
    }

    pub fn get_block_device(&self, major: i16) -> &'static dyn BlockDevice {
        if major < 0 || major as usize >= self.nblkdev {
            panic!("Block Device Doesn't Exist!");
        }

        block_device_for_major(major).expect("Block Device Doesn't Exist!")
    }

    pub fn n_char_devices(&self) -> usize {
        self.nchrdev
    }

    pub fn get_char_device(&self, major: i16) -> &'static dyn CharDevice {
        if major < 0 || major as usize >= self.nchrdev {
            panic!("Char Device Doesn't Exist!");
        }

        char_device_for_major(major).expect("Char Device Doesn't Exist!")
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_DEVICE_MANAGER: LazyLock<DeviceManager> =
    LazyLock::new(DeviceManager::new);

pub fn global_device_manager() -> &'static DeviceManager {
    &GLOBAL_DEVICE_MANAGER
}
