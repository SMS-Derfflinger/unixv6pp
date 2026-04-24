use crate::{
    dev::buffer_manager::global_buffer_manager,
    sync::{IrqGuard, SuperCell},
};
use eonix_sync_base::LazyLock;

use super::{
    ata_driver::ATADriver,
    buffer::{BufDeviceAdapter, BufDeviceList, BufFlag, BufIoAdapter, BufIoQueue, BufRef},
    device_manager::major,
};

#[repr(C)]
pub struct Devtab {
    pub d_active: i32,
    pub d_errcnt: i32,
    pub buffer_list: BufDeviceList,
    pub io_queue: BufIoQueue,
}

unsafe impl Send for Devtab {}

impl Devtab {
    pub const fn new() -> Self {
        Self {
            d_active: 0,
            d_errcnt: 0,
            buffer_list: BufDeviceList::new(BufDeviceAdapter::NEW),
            io_queue: BufIoQueue::new(BufIoAdapter::NEW),
        }
    }

    pub fn pop_io_request(&mut self) -> Option<BufRef> {
        self.io_queue.pop_front().map(BufRef::from_unsafe_ref)
    }

    pub fn peek_io_request(&self) -> Option<BufRef> {
        self.io_queue
            .front()
            .clone_pointer()
            .map(BufRef::from_unsafe_ref)
    }

    pub fn push_io_request(&mut self, bp: BufRef) {
        if bp.as_ref().is_on_io_queue() {
            return;
        }

        self.io_queue.push_back(bp.into_unsafe_ref());
    }
}

pub trait BlockDevice: Sync {
    fn open(&self, dev: i16, mode: i32) -> i32;
    fn close(&self, dev: i16, mode: i32) -> i32;
    fn strategy(&self, bp: BufRef) -> i32;
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

    fn strategy(&self, bp: BufRef) -> i32 {
        if bp.as_ref().b_blkno.0 >= Self::NSECTOR {
            bp.as_mut().b_flags.insert(BufFlag::B_ERROR);
            global_buffer_manager().io_done(bp);
            return 0;
        }

        let should_start = {
            let ctx = IrqGuard::disable_save();
            let should_start = self.tab.with_mut(|tab| {
                tab.push_io_request(bp);
                tab.d_active == 0
            });
            drop(ctx);
            should_start
        };

        if should_start {
            self.start();
        }

        0
    }

    fn start(&self) {
        let bp = self.tab.with_mut(|tab| {
            if tab.d_active != 0 {
                return None;
            }

            let Some(bp) = tab.peek_io_request() else {
                return None;
            };

            tab.d_active = 1;
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
