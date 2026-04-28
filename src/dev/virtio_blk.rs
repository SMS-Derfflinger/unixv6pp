use core::{
    mem::{align_of, size_of},
    ptr::{addr_of, read_volatile, write_volatile},
    sync::atomic::{fence, Ordering},
};

use eonix_mm::address::Addr;

use crate::{
    constants::platform::{
        KERNEL_VIRT_BASE, RAM_BASE, VIRTIO_MMIO_BASE, VIRTIO_MMIO_COUNT, VIRTIO_MMIO_STRIDE,
    },
    dev::buffer::{BufFlag, BufRef},
    mm::{phys_to_virt, KernelPages},
    println_info,
    println_warn,
};

const VIRTIO_MAGIC: u32 = 0x7472_6976;
const VIRTIO_VERSION_MODERN: u32 = 2;
const VIRTIO_DEVICE_BLOCK: u32 = 2;
const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;
const VIRTIO_STATUS_DRIVER: u32 = 2;
const VIRTIO_STATUS_DRIVER_OK: u32 = 4;
const VIRTIO_STATUS_FEATURES_OK: u32 = 8;
const VIRTIO_STATUS_FAILED: u32 = 128;
const VIRTIO_F_VERSION_1: u64 = 1 << 32;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTQ_QUEUE_SIZE: u16 = 8;
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;
const VIRTIO_BLK_S_OK: u8 = 0;

const REG_MAGIC_VALUE: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_DEVICE_FEATURES: usize = 0x010;
const REG_DEVICE_FEATURES_SEL: usize = 0x014;
const REG_DRIVER_FEATURES: usize = 0x020;
const REG_DRIVER_FEATURES_SEL: usize = 0x024;
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_QUEUE_NUM: usize = 0x038;
const REG_QUEUE_READY: usize = 0x044;
const REG_QUEUE_NOTIFY: usize = 0x050;
const REG_INTERRUPT_ACK: usize = 0x064;
const REG_STATUS: usize = 0x070;
const REG_QUEUE_DESC_LOW: usize = 0x080;
const REG_QUEUE_DESC_HIGH: usize = 0x084;
const REG_QUEUE_DRIVER_LOW: usize = 0x090;
const REG_QUEUE_DRIVER_HIGH: usize = 0x094;
const REG_QUEUE_DEVICE_LOW: usize = 0x0a0;
const REG_QUEUE_DEVICE_HIGH: usize = 0x0a4;
const REG_CONFIG_CAPACITY_LOW: usize = 0x100;
const REG_CONFIG_CAPACITY_HIGH: usize = 0x104;

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct VirtqAvail<const N: usize> {
    flags: u16,
    idx: u16,
    ring: [u16; N],
    used_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[repr(C)]
struct VirtqUsed<const N: usize> {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; N],
    avail_event: u16,
}

#[repr(C)]
struct VirtIOBlkReqHeader {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

pub struct VirtIOBlockDriver {
    mmio_base: usize,
    queue_pages: Option<KernelPages>,
    desc: *mut VirtqDesc,
    avail: *mut VirtqAvail<{ VIRTQ_QUEUE_SIZE as usize }>,
    used: *mut VirtqUsed<{ VIRTQ_QUEUE_SIZE as usize }>,
    request_header: *mut VirtIOBlkReqHeader,
    request_status: *mut u8,
    capacity: u64,
    last_used_idx: u16,
    initialized: bool,
}

unsafe impl Send for VirtIOBlockDriver {}
unsafe impl Sync for VirtIOBlockDriver {}

impl VirtIOBlockDriver {
    pub fn new() -> Self {
        Self {
            mmio_base: 0,
            queue_pages: None,
            desc: core::ptr::null_mut(),
            avail: core::ptr::null_mut(),
            used: core::ptr::null_mut(),
            request_header: core::ptr::null_mut(),
            request_status: core::ptr::null_mut(),
            capacity: 0,
            last_used_idx: 0,
            initialized: false,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn ensure_init(&mut self) -> Result<(), ()> {
        if self.initialized {
            return Ok(());
        }

        let Some((mmio_base, version)) = self.find_block_device() else {
            println_warn!(
                "virtio-blk: no supported block device found in {} MMIO slots",
                VIRTIO_MMIO_COUNT
            );
            return Err(());
        };
        self.mmio_base = mmio_base;
        if version != VIRTIO_VERSION_MODERN {
            println_warn!(
                "virtio-blk: unsupported transport version {} at {:#x}",
                version,
                mmio_base
            );
            return Err(());
        }

        self.write_reg(REG_STATUS, 0);
        self.write_reg(REG_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
        self.write_reg(
            REG_STATUS,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER,
        );

        self.write_reg(REG_DEVICE_FEATURES_SEL, 0);
        let _device_features_lo = self.read_reg(REG_DEVICE_FEATURES);
        self.write_reg(REG_DEVICE_FEATURES_SEL, 1);
        let device_features_hi = self.read_reg(REG_DEVICE_FEATURES);
        if (device_features_hi & 1) == 0 {
            self.fail();
            return Err(());
        }

        self.write_reg(REG_DRIVER_FEATURES_SEL, 0);
        self.write_reg(REG_DRIVER_FEATURES, 0);
        self.write_reg(REG_DRIVER_FEATURES_SEL, 1);
        self.write_reg(REG_DRIVER_FEATURES, 1);

        self.write_reg(
            REG_STATUS,
            VIRTIO_STATUS_ACKNOWLEDGE
                | VIRTIO_STATUS_DRIVER
                | VIRTIO_STATUS_FEATURES_OK,
        );

        if (self.read_reg(REG_STATUS) & VIRTIO_STATUS_FEATURES_OK) == 0 {
            self.fail();
            return Err(());
        }

        self.setup_queue()?;

        self.capacity = self.read_capacity();
        self.write_reg(
            REG_STATUS,
            VIRTIO_STATUS_ACKNOWLEDGE
                | VIRTIO_STATUS_DRIVER
                | VIRTIO_STATUS_FEATURES_OK
                | VIRTIO_STATUS_DRIVER_OK,
        );
        println_info!(
            "virtio-blk: initialized at {:#x} capacity={} sectors",
            self.mmio_base,
            self.capacity
        );

        self.initialized = true;
        Ok(())
    }

    pub fn transfer(&mut self, bp: BufRef) -> Result<(), ()> {
        self.ensure_init()?;

        let request_type = if bp.as_ref().b_flags.contains(BufFlag::B_READ) {
            VIRTIO_BLK_T_IN
        } else {
            VIRTIO_BLK_T_OUT
        };

        let data_ptr = bp.as_ref().io_addr();
        let data_len = bp.as_ref().b_wcount as usize;
        let header = unsafe { &mut *self.request_header };
        let status = unsafe { &mut *self.request_status };
        *header = VirtIOBlkReqHeader {
            request_type,
            reserved: 0,
            sector: bp.as_ref().b_blkno.0 as u64,
        };
        *status = 0xff;

        let header_paddr = pseudo_phys_to_bus(kernel_ptr_to_pseudo_phys(
            self.request_header.cast(),
        ));
        let data_paddr = pseudo_phys_to_bus(kernel_ptr_to_pseudo_phys(data_ptr));
        let status_paddr = pseudo_phys_to_bus(kernel_ptr_to_pseudo_phys(
            self.request_status.cast(),
        ));

        unsafe {
            self.desc.add(0).write(VirtqDesc {
                addr: header_paddr,
                len: size_of::<VirtIOBlkReqHeader>() as u32,
                flags: VIRTQ_DESC_F_NEXT,
                next: 1,
            });
            self.desc.add(1).write(VirtqDesc {
                addr: data_paddr,
                len: data_len as u32,
                flags: if request_type == VIRTIO_BLK_T_IN {
                    VIRTQ_DESC_F_NEXT | VIRTQ_DESC_F_WRITE
                } else {
                    VIRTQ_DESC_F_NEXT
                },
                next: 2,
            });
            self.desc.add(2).write(VirtqDesc {
                addr: status_paddr,
                len: 1,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            });

            let avail = &mut *self.avail;
            let slot = (avail.idx % VIRTQ_QUEUE_SIZE) as usize;
            avail.ring[slot] = 0;
            fence(Ordering::SeqCst);
            avail.idx = avail.idx.wrapping_add(1);
        }

        fence(Ordering::SeqCst);
        self.write_reg(REG_QUEUE_NOTIFY, 0);

        while unsafe { read_volatile(addr_of!((*self.used).idx)) } == self.last_used_idx {}
        self.last_used_idx = unsafe { read_volatile(addr_of!((*self.used).idx)) };

        self.write_reg(REG_INTERRUPT_ACK, 0xffff_ffff);
        if unsafe { read_volatile(self.request_status) } == VIRTIO_BLK_S_OK {
            Ok(())
        } else {
            println_warn!(
                "virtio-blk: request failed type={} sector={} len={} status={:#x} used_idx={}",
                request_type,
                header.sector,
                data_len,
                unsafe { read_volatile(self.request_status) },
                self.last_used_idx
            );
            Err(())
        }
    }

    fn setup_queue(&mut self) -> Result<(), ()> {
        self.write_reg(REG_QUEUE_SEL, 0);
        if self.read_reg(REG_QUEUE_NUM_MAX) < VIRTQ_QUEUE_SIZE as u32 {
            self.fail();
            return Err(());
        }

        let pages = KernelPages::alloc(0);
        let queue_base = phys_to_virt(pages.phys());
        unsafe {
            queue_base.write_bytes(0, 4096);
        }

        let desc = queue_base.cast::<VirtqDesc>();
        let avail_offset = size_of::<VirtqDesc>() * VIRTQ_QUEUE_SIZE as usize;
        let avail = unsafe { queue_base.add(avail_offset) }
            .cast::<VirtqAvail<{ VIRTQ_QUEUE_SIZE as usize }>>();
        let used_offset =
            align_up(avail_offset + size_of::<VirtqAvail<{ VIRTQ_QUEUE_SIZE as usize }>>(), align_of::<VirtqUsed<{ VIRTQ_QUEUE_SIZE as usize }>>());
        let used = unsafe { queue_base.add(used_offset) }
            .cast::<VirtqUsed<{ VIRTQ_QUEUE_SIZE as usize }>>();

        let request_offset = align_up(
            used_offset + size_of::<VirtqUsed<{ VIRTQ_QUEUE_SIZE as usize }>>(),
            align_of::<VirtIOBlkReqHeader>(),
        );
        let status_offset = request_offset + size_of::<VirtIOBlkReqHeader>();

        let request_header =
            unsafe { queue_base.add(request_offset) }.cast::<VirtIOBlkReqHeader>();
        let request_status = unsafe { queue_base.add(status_offset) };

        let desc_paddr = pseudo_phys_to_bus(pages.phys().addr()) as usize;
        let avail_paddr = desc_paddr + avail_offset;
        let used_paddr = desc_paddr + used_offset;

        self.write_reg(REG_QUEUE_NUM, VIRTQ_QUEUE_SIZE as u32);
        self.write_reg(REG_QUEUE_DESC_LOW, desc_paddr as u32);
        self.write_reg(REG_QUEUE_DESC_HIGH, (desc_paddr >> 32) as u32);
        self.write_reg(REG_QUEUE_DRIVER_LOW, avail_paddr as u32);
        self.write_reg(REG_QUEUE_DRIVER_HIGH, (avail_paddr >> 32) as u32);
        self.write_reg(REG_QUEUE_DEVICE_LOW, used_paddr as u32);
        self.write_reg(REG_QUEUE_DEVICE_HIGH, (used_paddr >> 32) as u32);
        self.write_reg(REG_QUEUE_READY, 1);

        self.queue_pages = Some(pages);
        self.desc = desc;
        self.avail = avail;
        self.used = used;
        self.request_header = request_header;
        self.request_status = request_status;
        self.last_used_idx = 0;
        Ok(())
    }

    fn read_capacity(&self) -> u64 {
        let low = self.read_reg(REG_CONFIG_CAPACITY_LOW) as u64;
        let high = self.read_reg(REG_CONFIG_CAPACITY_HIGH) as u64;
        (high << 32) | low
    }

    fn fail(&self) {
        self.write_reg(
            REG_STATUS,
            self.read_reg(REG_STATUS) | VIRTIO_STATUS_FAILED,
        );
    }

    fn find_block_device(&self) -> Option<(usize, u32)> {
        for index in 0..VIRTIO_MMIO_COUNT {
            let base = VIRTIO_MMIO_BASE + index * VIRTIO_MMIO_STRIDE;
            let magic = self.read_reg_at(base, REG_MAGIC_VALUE);
            let version = self.read_reg_at(base, REG_VERSION);
            let device_id = self.read_reg_at(base, REG_DEVICE_ID);

            if magic != VIRTIO_MAGIC {
                continue;
            }
            if device_id != VIRTIO_DEVICE_BLOCK {
                continue;
            }

            return Some((base, version));
        }

        None
    }

    fn read_reg(&self, offset: usize) -> u32 {
        self.read_reg_at(self.mmio_base, offset)
    }

    fn write_reg(&self, offset: usize, value: u32) {
        unsafe { write_volatile((self.mmio_base + offset) as *mut u32, value) }
    }

    fn read_reg_at(&self, base: usize, offset: usize) -> u32 {
        unsafe { read_volatile((base + offset) as *const u32) }
    }
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

fn pseudo_phys_to_bus(paddr: usize) -> u64 {
    (RAM_BASE + paddr) as u64
}

fn kernel_ptr_to_pseudo_phys(ptr: *mut u8) -> usize {
    let addr = ptr.addr();
    if addr >= KERNEL_VIRT_BASE {
        addr - KERNEL_VIRT_BASE
    } else {
        addr - RAM_BASE
    }
}
