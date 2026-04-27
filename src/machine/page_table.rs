use core::ops::{Index, IndexMut};

use bitflags::bitflags;
use eonix_mm::paging::PFN;
use riscv::{
    asm::sfence_vma_all,
    register::satp::{self, Mode},
};

use crate::{
    constants::platform::RAM_BASE,
};

const PAGE_SIZE: usize = 0x1000;
const ENTRY_COUNT_PER_PAGE_TABLE: usize = 512;
const SIZE_PER_USER_PAGE_TABLE_MAP: usize = ENTRY_COUNT_PER_PAGE_TABLE * PAGE_SIZE;
const SIZE_PER_DIRECTORY_MAP: usize = ENTRY_COUNT_PER_PAGE_TABLE * SIZE_PER_USER_PAGE_TABLE_MAP;
const SIZE_PER_KERNEL_ENTRY_MAP: usize = 0x20_0000;
const KERNEL_SPACE_START_ADDRESS: usize = 0xc000_0000;
const KERNEL_IDENTITY_BASE: usize = RAM_BASE;
const KERNEL_PAGED_WINDOW_BASE: usize = 0x0020_0000;
const KERNEL_PAGED_WINDOW_INDEX: usize = KERNEL_PAGED_WINDOW_BASE / SIZE_PER_KERNEL_ENTRY_MAP;
const KERNEL_UAREA_PTE_INDEX: usize = ENTRY_COUNT_PER_PAGE_TABLE - 1;
const PHYS_MEM_SIZE: usize = 64 * 1024 * 1024;

const ROOT_INDEX_LOW: usize = 0;
const ROOT_INDEX_KERNEL_IDENTITY: usize = KERNEL_IDENTITY_BASE / SIZE_PER_DIRECTORY_MAP;
const ROOT_INDEX_KERNEL_HIGH: usize = KERNEL_SPACE_START_ADDRESS / SIZE_PER_DIRECTORY_MAP;

const UART0_L1_INDEX: usize = 0x1000_0000 / SIZE_PER_KERNEL_ENTRY_MAP;
const PLIC_L1_INDEX: usize = 0x0c00_0000 / SIZE_PER_KERNEL_ENTRY_MAP;

pub const USER_PAGE_TABLE_COUNT: usize = 4;
pub const USER_SPACE_SIZE: usize = USER_PAGE_TABLE_COUNT * SIZE_PER_USER_PAGE_TABLE_MAP;

const MACHINE_PFN_BASE: usize = RAM_BASE >> 12;

const PTE_V: u64 = 1 << 0;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const PTE_G: u64 = 1 << 5;
const PTE_A: u64 = 1 << 6;
const PTE_D: u64 = 1 << 7;
const PPN_SHIFT: usize = 10;

pub fn global_user_page_table() -> &'static mut [PageTable; USER_PAGE_TABLE_COUNT] {
    user_page_table_array_mut()
}

pub trait HasUserStructAddress {
    fn user_struct_address(&self) -> usize;
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct EntryFlags: u64 {
        const PRESENT = 1 << 0;
        const WRITE = 1 << 1;
        const USER = 1 << 2;
        const LARGE = 1 << 7;
        const EXECUTE = 1 << 8;
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageDirectoryEntry(u64);

impl PageDirectoryEntry {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn set_table_ptr<T>(&mut self, table: *const T) {
        self.0 = encode_table_ptr(table);
    }

    pub fn set_large_identity_raw(&mut self, paddr: usize, flags: EntryFlags) {
        self.0 = encode_large_leaf_raw(paddr, flags);
    }
}

#[repr(C, align(4096))]
#[derive(Clone, Copy)]
pub struct PageDirectory {
    entries: [PageDirectoryEntry; ENTRY_COUNT_PER_PAGE_TABLE],
}

impl PageDirectory {
    pub const fn new() -> Self {
        Self {
            entries: [PageDirectoryEntry::new(); ENTRY_COUNT_PER_PAGE_TABLE],
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn set(&mut self, pfn: Option<PFN>, flags: EntryFlags) {
        self.0 = pfn
            .map(|pfn| encode_leaf_entry_pseudo(usize::from(pfn) << 12, flags))
            .unwrap_or(0);
    }

    pub fn get(&self) -> (PFN, EntryFlags) {
        let flags = decode_entry_flags(self.0);
        let pfn = if flags.contains(EntryFlags::PRESENT) {
            let machine_pfn = ((self.0 >> PPN_SHIFT) as usize).saturating_sub(MACHINE_PFN_BASE);
            PFN::from_val(machine_pfn)
        } else {
            PFN::from_val(0)
        };

        (pfn, flags)
    }
}

#[repr(C, align(4096))]
#[derive(Clone, Copy)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT_PER_PAGE_TABLE],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); ENTRY_COUNT_PER_PAGE_TABLE],
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

static mut ROOT_PAGE_DIRECTORY: PageDirectory = PageDirectory::new();
static mut LOW_PAGE_DIRECTORY: PageDirectory = PageDirectory::new();
static mut KERNEL_PAGE_DIRECTORY: PageDirectory = PageDirectory::new();
static mut KERNEL_PAGE_TABLE: PageTable = PageTable::new();
static mut USER_PAGE_TABLES: [PageTable; USER_PAGE_TABLE_COUNT] = [PageTable::new(); USER_PAGE_TABLE_COUNT];

pub fn init_page_directory() {
    let root = page_directory_mut();
    let low = low_page_directory_mut();
    let kernel = kernel_page_directory_mut();
    let kernel_leaf = kernel_page_table_mut();

    for entry in &mut root.entries {
        entry.clear();
    }
    for entry in &mut low.entries {
        entry.clear();
    }
    for entry in &mut kernel.entries {
        entry.clear();
    }
    for entry in kernel_leaf.iter_mut() {
        entry.set(None, EntryFlags::empty());
    }

    root.entries[ROOT_INDEX_LOW].set_table_ptr(low as *const _);
    root.entries[ROOT_INDEX_KERNEL_IDENTITY].set_table_ptr(kernel as *const _);
    root.entries[ROOT_INDEX_KERNEL_HIGH].set_table_ptr(kernel as *const _);

    init_kernel_linear_map(kernel, kernel_leaf);
    init_low_mmio_map(low);
}

#[no_mangle]
pub extern "C" fn init_user_page_table() {
    let low = low_page_directory_mut();

    for (idx, table) in user_page_table_array_mut().iter_mut().enumerate() {
        for entry in table.iter_mut() {
            entry.set(None, EntryFlags::USER);
        }

        low.entries[idx].set_table_ptr(table as *const _);
    }
}

pub fn enable_page_protection() {
    unsafe {
        satp::set(
            Mode::Sv39,
            0,
            (page_directory_mut() as *mut PageDirectory as usize) >> 12,
        );
    }
    flush_tlb();
}

pub fn flush_tlb() {
    sfence_vma_all()
}

#[no_mangle]
pub extern "C" fn _page_directory() -> *mut PageDirectory {
    page_directory_mut()
}

#[no_mangle]
pub extern "C" fn _kernel_page_table() -> *mut PageTable {
    kernel_page_table_mut()
}

#[no_mangle]
pub extern "C" fn _user_page_table_array() -> *mut PageTable {
    user_page_table_array_mut().as_mut_ptr().cast()
}

pub fn switch_user_struct<T: HasUserStructAddress>(proc: &T) {
    let pfn = PFN::from_val(proc.user_struct_address() >> 12);
    kernel_page_table_mut()[KERNEL_UAREA_PTE_INDEX]
        .set(Some(pfn), EntryFlags::PRESENT | EntryFlags::WRITE);
    flush_tlb();
}

pub fn page_directory_mut() -> &'static mut PageDirectory {
    #[allow(static_mut_refs)]
    unsafe {
        &mut ROOT_PAGE_DIRECTORY
    }
}

fn kernel_page_directory_mut() -> &'static mut PageDirectory {
    #[allow(static_mut_refs)]
    unsafe {
        &mut KERNEL_PAGE_DIRECTORY
    }
}

pub fn kernel_page_table_mut() -> &'static mut PageTable {
    #[allow(static_mut_refs)]
    unsafe {
        &mut KERNEL_PAGE_TABLE
    }
}

fn low_page_directory_mut() -> &'static mut PageDirectory {
    #[allow(static_mut_refs)]
    unsafe {
        &mut LOW_PAGE_DIRECTORY
    }
}

fn user_page_table_array_mut() -> &'static mut [PageTable; USER_PAGE_TABLE_COUNT] {
    #[allow(static_mut_refs)]
    unsafe {
        &mut USER_PAGE_TABLES
    }
}

fn init_kernel_linear_map(kernel: &mut PageDirectory, kernel_leaf: &mut PageTable) {
    let entry_count = PHYS_MEM_SIZE.div_ceil(SIZE_PER_KERNEL_ENTRY_MAP);

    for index in 0..entry_count {
        if index == KERNEL_PAGED_WINDOW_INDEX {
            for (pte_idx, pte) in kernel_leaf.iter_mut().enumerate() {
                let paddr = KERNEL_PAGED_WINDOW_BASE + pte_idx * PAGE_SIZE;
                pte.set(
                    Some(PFN::from_val(paddr >> 12)),
                    EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::EXECUTE,
                );
            }

            kernel.entries[index].set_table_ptr(kernel_leaf as *const _);
        } else {
            let paddr = RAM_BASE + index * SIZE_PER_KERNEL_ENTRY_MAP;
            kernel.entries[index].set_large_identity_raw(
                paddr,
                EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::LARGE | EntryFlags::EXECUTE,
            );
        }
    }
}

fn init_low_mmio_map(low: &mut PageDirectory) {
    low.entries[PLIC_L1_INDEX].set_large_identity_raw(
        0x0c00_0000,
        EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::LARGE,
    );
    low.entries[UART0_L1_INDEX].set_large_identity_raw(
        0x1000_0000,
        EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::LARGE,
    );
}

fn encode_table_ptr<T>(table: *const T) -> u64 {
    let pfn = (table as usize) >> 12;
    ((pfn as u64) << PPN_SHIFT) | PTE_V
}

fn encode_large_leaf_raw(paddr: usize, flags: EntryFlags) -> u64 {
    let pfn = paddr >> 12;
    ((pfn as u64) << PPN_SHIFT) | encode_leaf_flags(flags)
}

fn encode_leaf_entry_pseudo(paddr: usize, flags: EntryFlags) -> u64 {
    let machine_paddr = RAM_BASE + paddr;
    encode_large_leaf_raw(machine_paddr, flags)
}

fn encode_leaf_flags(flags: EntryFlags) -> u64 {
    if !flags.contains(EntryFlags::PRESENT) {
        return 0;
    }

    let mut bits = PTE_V | PTE_R | PTE_A;

    if flags.contains(EntryFlags::WRITE) {
        bits |= PTE_W | PTE_D;
    }

    if flags.contains(EntryFlags::USER) {
        bits |= PTE_U;
        if !flags.contains(EntryFlags::WRITE) || flags.contains(EntryFlags::EXECUTE) {
            bits |= PTE_X;
        }
    } else if flags.contains(EntryFlags::EXECUTE) {
        bits |= PTE_X;
    }

    if flags.contains(EntryFlags::LARGE) {
        bits |= PTE_G;
    }

    bits
}

fn decode_entry_flags(entry: u64) -> EntryFlags {
    let mut flags = EntryFlags::empty();

    if entry & PTE_V != 0 {
        flags |= EntryFlags::PRESENT;
    }
    if entry & PTE_W != 0 {
        flags |= EntryFlags::WRITE;
    }
    if entry & PTE_U != 0 {
        flags |= EntryFlags::USER;
    }
    if entry & PTE_X != 0 {
        flags |= EntryFlags::EXECUTE;
    }

    flags
}
