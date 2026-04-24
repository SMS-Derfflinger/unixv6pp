use crate::{dev::buffer_manager::global_buffer_manager, sync::SuperCell};

use super::{
    block_device::{ata_block_device, BlockDevice},
    buffer::{Buf, BufFlag, BufRef},
    device_manager::minor,
    dma::{DMAType, PRDTable, PhysicalRegionDescriptor, DMA},
    io_port::IOPort,
};

pub struct ATADriver;

impl ATADriver {
    pub const DATA_PORT: u16 = 0x1f0;
    pub const ERROR_PORT: u16 = 0x1f1;
    pub const NSECTOR_PORT: u16 = 0x1f2;
    pub const BLKNO_PORT_1: u16 = 0x1f3;
    pub const BLKNO_PORT_2: u16 = 0x1f4;
    pub const BLKNO_PORT_3: u16 = 0x1f5;
    pub const MODE_PORT: u16 = 0x1f6;
    pub const CMD_PORT: u16 = 0x1f7;
    pub const STATUS_PORT: u16 = 0x1f7;
    pub const CTRL_PORT: u16 = 0x3f6;

    pub const HD_ERROR: u8 = 0x01;
    pub const HD_DEVICE_REQUEST: u8 = 0x08;
    pub const HD_DEVICE_FAULT: u8 = 0x20;
    pub const HD_DEVICE_READY: u8 = 0x40;
    pub const HD_DEVICE_BUSY: u8 = 0x80;

    pub const HD_READ: u8 = 0x20;
    pub const HD_WRITE: u8 = 0x30;
    pub const HD_DMA_READ: u8 = 0xc8;
    pub const HD_DMA_WRITE: u8 = 0xca;

    pub const MODE_IDE: u8 = 0xa0;
    pub const MODE_LBA28: u8 = 0x40;

    pub const MASTER_PIC_COMMAND_PORT: u16 = 0x20;
    pub const SLAVE_PIC_COMMAND_PORT: u16 = 0xa0;
    pub const PIC_EOI: u8 = 0x20;

    pub fn ata_handler() {
        enum HandlerAction {
            Ignore,
            Retry,
            Complete(BufRef),
        }

        let bdev = ata_block_device();
        let action = bdev.devtab().with_mut(|tab| {
            if tab.d_active == 0 {
                return HandlerAction::Ignore;
            }

            let Some(bp) = tab.peek_io_request() else {
                tab.d_active = 0;
                return HandlerAction::Ignore;
            };
            tab.d_active = 0;

            if Self::is_error() || DMA::is_error() {
                tab.d_errcnt += 1;
                if tab.d_errcnt <= 10 {
                    return HandlerAction::Retry;
                }

                bp.as_mut().b_flags.insert(BufFlag::B_ERROR);
            }

            tab.d_errcnt = 0;
            let _ = tab.pop_io_request();
            HandlerAction::Complete(bp)
        });

        match action {
            HandlerAction::Ignore => {}
            HandlerAction::Retry => {
                bdev.start();
            }
            HandlerAction::Complete(bp) => {
                global_buffer_manager().io_done(bp);
                let _ = bp;
                bdev.start();
            }
        }

        Self::send_disk_eoi();
    }

    pub fn dev_start(bp: BufRef) {
        if Self::is_controller_ready() == 0 {
            panic!("Disk Hang Up!");
        }

        let bp_ref = bp.as_mut();
        let minor = minor(bp_ref.b_dev) as u8;
        let blkno = bp_ref.b_blkno.0;
        let sectors = (bp_ref.b_wcount as usize / Buf::BLOCK_SIZE) as u8;

        let mut prd = PhysicalRegionDescriptor::new();
        prd.set_base_address((bp_ref.io_addr() as usize & !0xc000_0000) as u32);
        prd.set_byte_count(bp_ref.b_wcount as u16);

        let table_base = PRD_TABLE.with_mut(|table| {
            table.set_physical_region_descriptor(0, prd, true);
            table.prd_table_base_address()
        });

        DMA::reset();

        unsafe {
            IOPort::out_byte(Self::NSECTOR_PORT, sectors);
            IOPort::out_byte(Self::BLKNO_PORT_1, (blkno & 0xff) as u8);
            IOPort::out_byte(Self::BLKNO_PORT_2, ((blkno >> 8) & 0xff) as u8);
            IOPort::out_byte(Self::BLKNO_PORT_3, ((blkno >> 16) & 0xff) as u8);
            IOPort::out_byte(
                Self::MODE_PORT,
                Self::MODE_IDE | Self::MODE_LBA28 | (minor << 4) | ((blkno >> 24) & 0x0f) as u8,
            );

            if bp_ref.b_flags.contains(BufFlag::B_READ) {
                DMA::start(DMAType::Read, table_base);
                IOPort::out_byte(Self::CMD_PORT, Self::HD_DMA_READ);
            } else {
                DMA::start(DMAType::Write, table_base);
                IOPort::out_byte(Self::CMD_PORT, Self::HD_DMA_WRITE);
            }
        }
    }

    fn is_controller_ready() -> i32 {
        let mut ticks = 10000;

        while ticks > 0 {
            ticks -= 1;
            let status = unsafe { IOPort::in_byte(Self::STATUS_PORT) };
            if status & (Self::HD_DEVICE_BUSY | Self::HD_DEVICE_READY) == Self::HD_DEVICE_READY {
                return ticks;
            }
        }

        0
    }

    fn is_error() -> bool {
        unsafe { IOPort::in_byte(Self::STATUS_PORT) & Self::HD_ERROR == Self::HD_ERROR }
    }

    fn send_disk_eoi() {
        unsafe {
            IOPort::out_byte(Self::SLAVE_PIC_COMMAND_PORT, Self::PIC_EOI);
            IOPort::out_byte(Self::MASTER_PIC_COMMAND_PORT, Self::PIC_EOI);
        }
    }
}

static PRD_TABLE: SuperCell<PRDTable> = SuperCell::new(PRDTable::new());
