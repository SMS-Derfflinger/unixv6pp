use eonix_sync_base::LazyLock;

use crate::{
    constants::PosixError,
    dev::buffer_manager::global_buffer_manager,
    sync::{IrqGuard, SuperCell},
    user::Userspace,
};

use super::{
    ata_driver::ATADriver,
    buffer::{Buf, BufFlag},
    device_manager::major,
};

#[repr(C)]
pub struct Devtab {
    pub d_active: i32,
    pub d_errcnt: i32,
    pub b_forw: *mut Buf,
    pub b_back: *mut Buf,
    pub d_actf: *mut Buf,
    pub d_actl: *mut Buf,
}

unsafe impl Send for Devtab {}

impl Devtab {
    pub const fn new() -> Self {
        Self {
            d_active: 0,
            d_errcnt: 0,
            b_forw: core::ptr::null_mut(),
            b_back: core::ptr::null_mut(),
            d_actf: core::ptr::null_mut(),
            d_actl: core::ptr::null_mut(),
        }
    }

    pub fn ensure_buffer_list(&mut self) {
        if self.b_forw.is_null() || self.b_back.is_null() {
            let sentinel = self.sentinel();
            self.b_forw = sentinel;
            self.b_back = sentinel;
        }
    }

    pub fn pop_io_request(&mut self) -> Option<*mut Buf> {
        let bp = self.d_actf;
        if bp.is_null() {
            self.d_actl = core::ptr::null_mut();
            return None;
        }

        unsafe {
            self.d_actf = (*bp).av_forw;
            if self.d_actf.is_null() {
                self.d_actl = core::ptr::null_mut();
            }
            (*bp).av_forw = core::ptr::null_mut();
            (*bp).av_back = core::ptr::null_mut();
        }

        Some(bp)
    }

    pub fn peek_io_request(&self) -> Option<*mut Buf> {
        (!self.d_actf.is_null()).then_some(self.d_actf)
    }

    pub fn push_io_request(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            (*bp).av_forw = core::ptr::null_mut();
            (*bp).av_back = core::ptr::null_mut();

            if self.d_actf.is_null() {
                self.d_actf = bp;
            } else {
                (*self.d_actl).av_forw = bp;
            }
            self.d_actl = bp;
        }
    }

    pub fn sentinel(&mut self) -> *mut Buf {
        self as *mut Devtab as *mut Buf
    }

    pub fn sentinel_const(&self) -> *mut Buf {
        self as *const Devtab as *mut Buf
    }
}

pub trait BlockDevice: Sync {
    fn open(&self, dev: i16, mode: i32) -> i32;
    fn close(&self, dev: i16, mode: i32) -> i32;
    fn strategy(&self, bp: *mut Buf) -> i32;
    fn start(&self);
    fn devtab(&self) -> &SuperCell<Devtab>;
}

pub struct ATABlockDevice {
    tab: SuperCell<Devtab>,
}

impl ATABlockDevice {
    pub const NSECTOR: u32 = 0x7fff_ffff;

    pub fn new() -> Self {
        Self {
            tab: SuperCell::new(Devtab::new()),
        }
    }
}

impl BlockDevice for ATABlockDevice {
    fn open(&self, _dev: i16, _mode: i32) -> i32 {
        0
    }

    fn close(&self, _dev: i16, _mode: i32) -> i32 {
        0
    }

    fn strategy(&self, bp: *mut Buf) -> i32 {
        if bp.is_null() {
            return 0;
        }

        unsafe {
            if (*bp).b_blkno.0 >= Self::NSECTOR {
                (*bp).b_flags.insert(BufFlag::B_ERROR);
                global_buffer_manager().io_done(bp);
                return 0;
            }
        }

        let ctx = IrqGuard::disable_save();
        let should_start = self.tab.with_mut(|tab| {
            tab.push_io_request(bp);
            tab.d_active == 0
        });

        if should_start {
            self.start();
        }

        0
    }

    fn start(&self) {
        let bp = self.tab.with_mut(|tab| {
            let Some(bp) = tab.peek_io_request() else {
                return None;
            };
            tab.d_active += 1;
            Some(bp)
        });

        let Some(bp) = bp else {
            return;
        };

        ATADriver::dev_start(bp);
    }

    fn devtab(&self) -> &SuperCell<Devtab> {
        &self.tab
    }
}

static ATA_BLOCK_DEVICE: LazyLock<ATABlockDevice> = LazyLock::new(ATABlockDevice::new);

pub fn ata_block_device() -> &'static ATABlockDevice {
    &ATA_BLOCK_DEVICE
}

pub fn block_device_for_major(major: i16) -> Option<&'static dyn BlockDevice> {
    match major {
        0 => Some(ata_block_device()),
        _ => None,
    }
}

pub fn block_device_for_dev(dev: i16) -> Option<&'static dyn BlockDevice> {
    block_device_for_major(major(dev))
}

fn errno(err: PosixError) -> i32 {
    -(err as i32)
}

fn set_block_error(err: PosixError) -> i32 {
    Userspace::get().set_error(err);
    errno(err)
}

#[no_mangle]
pub extern "C" fn block_device_open(dev: i16, mode: i32) -> i32 {
    match block_device_for_dev(dev) {
        Some(device) => device.open(dev, mode),
        None => set_block_error(PosixError::ENXIO),
    }
}

#[no_mangle]
pub extern "C" fn block_device_close(dev: i16, mode: i32) -> i32 {
    match block_device_for_dev(dev) {
        Some(device) => device.close(dev, mode),
        None => set_block_error(PosixError::ENXIO),
    }
}

#[no_mangle]
pub extern "C" fn block_device_strategy(bp: *mut Buf) -> i32 {
    if bp.is_null() {
        return set_block_error(PosixError::EINVAL);
    }

    let dev = unsafe { (*bp).b_dev };
    match block_device_for_dev(dev) {
        Some(device) => device.strategy(bp),
        None => {
            unsafe {
                (*bp).b_flags.insert(BufFlag::B_ERROR);
            }
            global_buffer_manager().io_done(bp);
            set_block_error(PosixError::ENXIO)
        }
    }
}

#[no_mangle]
pub extern "C" fn block_device_start(major: i16) {
    match block_device_for_major(major) {
        Some(device) => device.start(),
        None => {
            set_block_error(PosixError::ENXIO);
        }
    }
}
