use core::{ptr::NonNull, slice};

use eonix_mm::address::Addr;
use virtio_drivers::{
    device::blk::{VirtIOBlk, SECTOR_SIZE},
    transport::{
        mmio::{MmioTransport, MmioVersion, VirtIOHeader},
        DeviceType, Transport,
    },
    BufferDirection, Hal, PhysAddr,
};

use crate::{
    constants::platform::{
        KERNEL_VIRT_BASE, RAM_BASE, VIRTIO_MMIO_BASE, VIRTIO_MMIO_COUNT, VIRTIO_MMIO_STRIDE,
    },
    dev::buffer::{BufFlag, BufRef},
    mm::{free_page, phys_to_virt, virt_to_phys, KernelPages, PAGE_SIZE},
    println_info, println_warn,
};

type VirtIOBlkDriverImpl = VirtIOBlk<VirtioHal, MmioTransport<'static>>;

pub struct VirtIOBlockDriver {
    mmio_base: usize,
    inner: Option<VirtIOBlkDriverImpl>,
}

unsafe impl Send for VirtIOBlockDriver {}
unsafe impl Sync for VirtIOBlockDriver {}

impl VirtIOBlockDriver {
    pub fn new() -> Self {
        Self {
            mmio_base: 0,
            inner: None,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.inner.as_ref().map_or(0, VirtIOBlk::capacity)
    }

    pub fn ensure_init(&mut self) -> Result<(), ()> {
        if self.inner.is_some() {
            return Ok(());
        }

        let Some((mmio_base, transport)) = self.find_block_device() else {
            println_warn!(
                "virtio-blk: no supported block device found in {} MMIO slots",
                VIRTIO_MMIO_COUNT
            );
            return Err(());
        };

        let driver = VirtIOBlk::<VirtioHal, _>::new(transport).map_err(|err| {
            println_warn!(
                "virtio-blk: failed to initialize device at {:#x}: {:?}",
                mmio_base,
                err
            );
        })?;

        println_info!(
            "virtio-blk: initialized at {:#x} capacity={} sectors",
            mmio_base,
            driver.capacity()
        );

        self.mmio_base = mmio_base;
        self.inner = Some(driver);
        Ok(())
    }

    pub fn transfer(&mut self, bp: BufRef) -> Result<(), ()> {
        self.ensure_init()?;

        let block_id = bp.as_ref().b_blkno.0 as usize;
        let data_len = bp.as_ref().b_wcount as usize;
        if data_len == 0 || data_len % SECTOR_SIZE != 0 {
            println_warn!(
                "virtio-blk: invalid request size block={} len={}",
                block_id,
                data_len
            );
            return Err(());
        }

        let driver = self.inner.as_mut().ok_or(())?;
        if bp.as_ref().b_flags.contains(BufFlag::B_READ) {
            let data = unsafe { slice::from_raw_parts_mut(bp.as_ref().io_addr(), data_len) };
            driver.read_blocks(block_id, data).map_err(|err| {
                println_warn!(
                    "virtio-blk: read failed block={} len={} err={:?}",
                    block_id,
                    data_len,
                    err
                );
            })
        } else {
            let data = unsafe { slice::from_raw_parts(bp.as_ref().io_addr(), data_len) };
            driver.write_blocks(block_id, data).map_err(|err| {
                println_warn!(
                    "virtio-blk: write failed block={} len={} err={:?}",
                    block_id,
                    data_len,
                    err
                );
            })
        }
    }

    fn find_block_device(&self) -> Option<(usize, MmioTransport<'static>)> {
        for index in 0..VIRTIO_MMIO_COUNT {
            let base = VIRTIO_MMIO_BASE + index * VIRTIO_MMIO_STRIDE;
            let header = NonNull::new(base as *mut VirtIOHeader).unwrap();
            let transport = unsafe { MmioTransport::new(header, VIRTIO_MMIO_STRIDE) };
            let Ok(transport) = transport else {
                continue;
            };

            if transport.version() != MmioVersion::Modern {
                println_warn!(
                    "virtio-blk: unsupported transport version {:?} at {:#x}",
                    transport.version(),
                    base
                );
                continue;
            }

            if transport.device_type() == DeviceType::Block {
                return Some((base, transport));
            }
        }

        None
    }
}

struct VirtioHal;

unsafe impl Hal for VirtioHal {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let alloc_pages = pages.max(1).next_power_of_two();
        let alloc = KernelPages::alloc_bytes(alloc_pages * PAGE_SIZE);
        let paddr = alloc.phys().addr();
        let vaddr = NonNull::new(phys_to_virt(alloc.phys())).unwrap();

        core::mem::forget(alloc);
        (pseudo_phys_to_bus(paddr), vaddr)
    }

    unsafe fn dma_dealloc(paddr: PhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        let alloc_pages = pages.max(1).next_power_of_two();
        free_page(bus_to_pseudo_phys(paddr), alloc_pages * PAGE_SIZE);
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        NonNull::new(paddr as *mut u8).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        pseudo_phys_to_bus(kernel_ptr_to_pseudo_phys(buffer.as_ptr().cast::<u8>()))
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}

fn pseudo_phys_to_bus(paddr: usize) -> usize {
    RAM_BASE + paddr
}

fn bus_to_pseudo_phys(paddr: usize) -> usize {
    paddr - RAM_BASE
}

fn kernel_ptr_to_pseudo_phys(ptr: *mut u8) -> usize {
    let addr = ptr.addr();
    if addr >= KERNEL_VIRT_BASE {
        virt_to_phys(ptr).addr()
    } else {
        addr - RAM_BASE
    }
}
