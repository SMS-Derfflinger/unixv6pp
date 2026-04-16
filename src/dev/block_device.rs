use core::arch::asm;

use eonix_spin::{NoContext, Spin};
use eonix_sync_base::LazyLock;
use intrusive_collections::{LinkedList, UnsafeRef};

use crate::sync::SpinExt;

use super::{
    ata_driver::ATADriver,
    buffer::{Buf, BufDeviceAdapter, BufFlag, BufIoAdapter},
    device_manager::major,
};

pub struct Devtab {
    pub d_active: i32,
    pub d_errcnt: i32,
    pub buffers: LinkedList<BufDeviceAdapter>,
    pub io_queue: LinkedList<BufIoAdapter>,
}

unsafe impl Send for Devtab {}

impl Devtab {
    pub const fn new() -> Self {
        Self {
            d_active: 0,
            d_errcnt: 0,
            buffers: LinkedList::new(BufDeviceAdapter::NEW),
            io_queue: LinkedList::new(BufIoAdapter::NEW),
        }
    }

    pub fn pop_io_request(&mut self) -> Option<*mut Buf> {
        self.io_queue
            .pop_front()
            .map(|buf| UnsafeRef::into_raw(buf) as *mut Buf)
    }

    pub fn peek_io_request(&self) -> Option<*mut Buf> {
        self.io_queue
            .front()
            .clone_pointer()
            .map(|buf| UnsafeRef::into_raw(buf) as *mut Buf)
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
        Self {
            tab: Spin::new(Devtab::new()),
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
                // TODO: BufferManager::IODone(bp)
                return 0;
            }
        }

        disable_interrupts();
        {
            let mut tab = self.tab.lock();
            let buf = unsafe { UnsafeRef::from_raw(bp as *const Buf) };
            tab.io_queue.push_back(buf);

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
            let Some(bp) = tab.peek_io_request() else {
                return;
            };
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
