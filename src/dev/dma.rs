use core::sync::atomic::{AtomicU16, AtomicU32, Ordering};

use super::io_port::IOPort;

const PCI_CONFIG_ADDRESS_PORT: u16 = 0x0cf8;
const PCI_CONFIG_DATA_PORT: u16 = 0x0cfc;
const PCI_ENABLE: u32 = 0x8000_0000;
const PCI_BUS_COUNT: u32 = 256;
const PCI_DEV_COUNT: u32 = 32;
const PCI_FUNC_COUNT: u32 = 8;
const PCI_CONFIG_DWORDS: usize = 64;
const PCI_CLASS_DWORD_INDEX: usize = 2;
const PCI_BUS_MASTER_DWORD_OFFSET: usize = 8;
const PCI_COMMAND_REGISTER_OFFSET: u32 = 0x04;
const PCI_IDE_CLASS_MASK: u32 = 0x0101_0000;
const PCI_IDE_CLASS_VALUE: u32 = 0x0101_0000;
const PCI_IO_SPACE_INDICATOR: u32 = 0x1;
const PCI_IO_BASE_MASK: u32 = 0xfffe;
const PCI_BUS_MASTER_ENABLE: u8 = 0x7;
const KERNEL_BASE: usize = 0xc000_0000;

pub static DMA_CONFIG: [AtomicU32; PCI_CONFIG_DWORDS] =
    [const { AtomicU32::new(0) }; PCI_CONFIG_DWORDS];

static COMMAND_PORT: AtomicU16 = AtomicU16::new(0);
static STATUS_PORT: AtomicU16 = AtomicU16::new(0);
static PRDTR_PORT: AtomicU16 = AtomicU16::new(0);

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct PhysicalRegionDescriptor {
    base_address: u32,
    byte_count: u16,
    flags: u16,
}

impl PhysicalRegionDescriptor {
    const EOT: u16 = 0x8000;

    pub const fn new() -> Self {
        Self {
            base_address: 0,
            byte_count: 0,
            flags: 0,
        }
    }

    pub fn set_base_address(&mut self, phy_base_addr: u32) {
        self.base_address = phy_base_addr & !0x1;
    }

    pub fn set_byte_count(&mut self, bytes: u16) {
        self.byte_count = bytes & !0x1;
    }

    pub fn set_end_of_table(&mut self, eot: bool) {
        if eot {
            self.flags |= Self::EOT;
        } else {
            self.flags &= !Self::EOT;
        }
    }

    pub fn base_address(&self) -> u32 {
        self.base_address
    }

    pub fn byte_count(&self) -> u16 {
        self.byte_count
    }

    pub fn is_end_of_table(&self) -> bool {
        self.flags & Self::EOT != 0
    }
}

impl Default for PhysicalRegionDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C, align(4))]
pub struct PRDTable {
    descriptors: [PhysicalRegionDescriptor; Self::NSIZE],
}

impl PRDTable {
    pub const NSIZE: usize = 10;

    pub const fn new() -> Self {
        Self {
            descriptors: [PhysicalRegionDescriptor::new(); Self::NSIZE],
        }
    }

    pub fn set_physical_region_descriptor(
        &mut self,
        index: usize,
        mut prd: PhysicalRegionDescriptor,
        eot: bool,
    ) {
        prd.set_end_of_table(eot);
        self.descriptors[index] = prd;
    }

    pub fn prd_table_base_address(&self) -> u32 {
        (self.descriptors.as_ptr() as usize).wrapping_sub(KERNEL_BASE) as u32
    }

    pub fn descriptor(&self, index: usize) -> &PhysicalRegionDescriptor {
        &self.descriptors[index]
    }
}

impl Default for PRDTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DMAStart {
    Start = 0x01,
    Stop = 0x00,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DMAType {
    Read = 0x08,
    Write = 0x00,
}

pub struct DMA;

impl DMA {
    pub const ACTIVE: u8 = 0x01;
    pub const ERROR: u8 = 0x02;
    pub const INTERRUPT: u8 = 0x04;

    pub fn init() {
        let mut config = [0u32; PCI_CONFIG_DWORDS];

        for bus in 0..PCI_BUS_COUNT {
            for dev in 0..PCI_DEV_COUNT {
                for func in 0..PCI_FUNC_COUNT {
                    let mut found_ide = false;

                    for index in 0..PCI_CONFIG_DWORDS {
                        let address = Self::pci_config_address(bus, dev, func, (index as u32) << 2);
                        unsafe {
                            IOPort::out_dword(PCI_CONFIG_ADDRESS_PORT, address);
                        }

                        let value = unsafe { IOPort::in_dword(PCI_CONFIG_DATA_PORT) };
                        config[index] = value;
                        DMA_CONFIG[index].store(value, Ordering::Relaxed);

                        if value == u32::MAX {
                            continue;
                        }

                        if index == PCI_CLASS_DWORD_INDEX
                            && (value & PCI_IDE_CLASS_MASK) == PCI_IDE_CLASS_VALUE
                        {
                            found_ide = true;
                        }
                    }

                    if found_ide {
                        Self::configure_from_pci_function(bus, dev, func, &mut config);
                        return;
                    }
                }
            }
        }
    }

    pub fn reset() {
        unsafe {
            IOPort::out_byte(Self::command_port(), DMAStart::Stop as u8);
            IOPort::out_byte(Self::status_port(), Self::INTERRUPT | Self::ERROR);
        }
    }

    pub fn is_error() -> bool {
        unsafe { IOPort::in_byte(Self::status_port()) & Self::ERROR == Self::ERROR }
    }

    pub fn start(dma_type: DMAType, base_address: u32) {
        unsafe {
            IOPort::out_dword(Self::prdtr_port(), base_address);
            IOPort::out_byte(
                Self::command_port(),
                (dma_type as u8) | (DMAStart::Start as u8),
            );
        }
    }

    pub fn command_port() -> u16 {
        COMMAND_PORT.load(Ordering::Relaxed)
    }

    pub fn status_port() -> u16 {
        STATUS_PORT.load(Ordering::Relaxed)
    }

    pub fn prdtr_port() -> u16 {
        PRDTR_PORT.load(Ordering::Relaxed)
    }

    fn configure_from_pci_function(bus: u32, dev: u32, func: u32, config: &mut [u32; 64]) {
        let bus_master_base = config[PCI_BUS_MASTER_DWORD_OFFSET];
        if bus_master_base & PCI_IO_SPACE_INDICATOR == 0 {
            panic!("Error: Unsupported Memory Mapped I/O");
        }

        let port_addr = (bus_master_base & PCI_IO_BASE_MASK) as u16;
        COMMAND_PORT.store(port_addr, Ordering::Relaxed);
        STATUS_PORT.store(port_addr + 2, Ordering::Relaxed);
        PRDTR_PORT.store(port_addr + 4, Ordering::Relaxed);

        let command_register =
            Self::pci_config_address(bus, dev, func, PCI_COMMAND_REGISTER_OFFSET);
        unsafe {
            IOPort::out_dword(PCI_CONFIG_ADDRESS_PORT, command_register);
            IOPort::out_byte(PCI_CONFIG_DATA_PORT, PCI_BUS_MASTER_ENABLE);
            IOPort::out_dword(PCI_CONFIG_ADDRESS_PORT, command_register);
        }

        let value = unsafe { IOPort::in_dword(PCI_CONFIG_DATA_PORT) };
        config[1] = value;
        DMA_CONFIG[1].store(value, Ordering::Relaxed);
    }

    fn pci_config_address(bus: u32, dev: u32, func: u32, offset: u32) -> u32 {
        PCI_ENABLE | (bus << 16) | (dev << 11) | (func << 8) | offset
    }
}
