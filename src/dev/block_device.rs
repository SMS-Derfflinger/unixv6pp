use core::{arch::asm, ptr};

use eonix_spin::{NoContext, Spin};
use eonix_sync_base::LazyLock;

use crate::sync::SpinExt;

use super::{
    ata_driver::ATADriver,
    buffer::{Buf, BufFlag},
    device_manager::major,
};

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
            b_forw: ptr::null_mut(),
            b_back: ptr::null_mut(),
            d_actf: ptr::null_mut(),
            d_actl: ptr::null_mut(),
        }
    }

    fn initialize_device_queue(&mut self) {
        let self_as_buf = self as *mut Self as *mut Buf;
        self.b_forw = self_as_buf;
        self.b_back = self_as_buf;
    }
}

pub trait BlockDevice: Sync {
    fn open(&self, dev: i16, mode: i32) -> i32;
    fn close(&self, dev: i16, mode: i32) -> i32;
    fn strategy(&self, bp: *mut Buf) -> i32;
    fn start(&self);
    fn devtab(&self) -> &Spin<Devtab>;
}

pub struct ATABlockDevice {
    tab: Spin<Devtab>,
}

impl ATABlockDevice {
    pub const NSECTOR: u32 = 0x7fff_ffff;

    pub fn new() -> Self {
        let device = Self {
            tab: Spin::new(Devtab::new()),
        };
        device.tab.lock().initialize_device_queue();
        device
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
                // TODO: BufferManager::IODone(bp)
                return 0;
            }

            (*bp).av_forw = ptr::null_mut();
        }

        disable_interrupts();
        {
            let mut tab = self.tab.lock();
            if tab.d_actf.is_null() {
                tab.d_actf = bp;
            } else {
                unsafe {
                    (*tab.d_actl).av_forw = bp;
                }
            }
            tab.d_actl = bp;

            if tab.d_active == 0 {
                drop(tab);
                self.start();
                enable_interrupts();
                return 0;
            }
        }
        enable_interrupts();

        0
    }

    fn start(&self) {
        let bp = {
            let mut tab = self.tab.lock_ctx::<NoContext>();
            let bp = tab.d_actf;
            if bp.is_null() {
                return;
            }
            tab.d_active += 1;
            bp
        };

        ATADriver::dev_start(bp);
    }

    fn devtab(&self) -> &Spin<Devtab> {
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

// TODO: only for x86
fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}
