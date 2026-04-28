use crate::{
    dev::buffer_manager::global_buffer_manager,
    sync::{IrqGuard, SuperCell},
};
use eonix_sync_base::LazyLock;

use super::{
    buffer::{Buf, BufDeviceAdapter, BufDeviceList, BufFlag, BufIoAdapter, BufIoQueue, BufRef},
    device_manager::major,
    virtio_blk::VirtIOBlockDriver,
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

    pub fn push_io_request(&mut self, bp: BufRef) {
        if bp.as_ref().is_on_io_queue() {
            #[cfg(feature = "debug_irq")]
            crate::println_debug!("Already on queue");
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

pub struct VirtIOBlockDevice {
    tab: SuperCell<Devtab>,
    inner: SuperCell<VirtIOBlockDriver>,
}

unsafe impl Send for VirtIOBlockDevice {}
unsafe impl Sync for VirtIOBlockDevice {}

impl VirtIOBlockDevice {
    pub fn new() -> Self {
        Self {
            tab: SuperCell::new(Devtab::new()),
            inner: SuperCell::new(VirtIOBlockDriver::new()),
        }
    }
}

impl BlockDevice for VirtIOBlockDevice {
    fn open(&self, _dev: i16, _mode: i32) -> i32 {
        0
    }

    fn close(&self, _dev: i16, _mode: i32) -> i32 {
        0
    }

    fn strategy(&self, bp: BufRef) -> i32 {
        let sectors = (bp.as_ref().b_wcount as usize).div_ceil(Buf::BLOCK_SIZE) as u64;
        let blkno = bp.as_ref().b_blkno.0 as u64;
        let out_of_range = self.inner.with_mut(|driver| {
            driver.ensure_init().is_err() || blkno + sectors > driver.capacity()
        });

        if out_of_range {
            bp.as_mut().b_flags.insert(BufFlag::B_ERROR);
            global_buffer_manager().io_done(bp);
            return 0;
        }

        let should_start = {
            let _ctx = IrqGuard::disable_save();
            self.tab.with_mut(|tab| {
                #[cfg(feature = "debug_irq")]
                crate::println_debug!("push request");
                tab.push_io_request(bp);
                tab.d_active == 0
            })
        };

        if should_start {
            self.start();
        }

        0
    }

    fn start(&self) {
        loop {
            let bp = self.tab.with_mut(|tab| {
                let _ctx = IrqGuard::disable_save();
                if tab.d_active != 0 {
                    return None;
                }

                let Some(bp) = tab.pop_io_request() else {
                    return None;
                };

                tab.d_active = 1;
                Some(bp)
            });

            let Some(bp) = bp else {
                return;
            };

            let ok = self.inner.with_mut(|driver| driver.transfer(bp).is_ok());

            self.tab.with_mut(|tab| {
                tab.d_active = 0;
                if !ok {
                    bp.as_mut().b_flags.insert(BufFlag::B_ERROR);
                }
            });

            global_buffer_manager().io_done(bp);
        }
    }

    fn devtab(&self) -> &SuperCell<Devtab> {
        &self.tab
    }
}

static VIRTIO_BLOCK_DEVICE: LazyLock<VirtIOBlockDevice> = LazyLock::new(VirtIOBlockDevice::new);

pub fn virtio_block_device() -> &'static VirtIOBlockDevice {
    &VIRTIO_BLOCK_DEVICE
}

pub fn block_device_for_major(major: i16) -> Option<&'static dyn BlockDevice> {
    match major {
        0 => Some(virtio_block_device()),
        _ => None,
    }
}

pub fn block_device_for_dev(dev: i16) -> Option<&'static dyn BlockDevice> {
    block_device_for_major(major(dev))
}
