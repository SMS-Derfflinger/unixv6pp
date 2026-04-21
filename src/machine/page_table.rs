use core::ops::{Index, IndexMut};

use bitflags::bitflags;
use eonix_mm::address::PAddr;
use eonix_mm::paging::PFN;

use crate::compat::compat_flush_page_directory;
use crate::mm::phys_to_virt;
use crate::proc::Process;

const ENTRY_COUNT_PER_PAGE_TABLE: usize = 1024;
const SIZE_PER_PAGE_TABLE_MAP: usize = 0x400000;
const PAGE_DIRECTORY_BASE_ADDRESS: usize = 0x200000;
const KERNEL_PAGE_TABLE_BASE_ADDRESS: usize = 0x201000;
pub const USER_PAGE_TABLE_BASE_ADDRESS: usize = 0x202000;
pub const USER_PAGE_TABLE_COUNT: usize = 2;
const KERNEL_SPACE_START_ADDRESS: usize = 0xc0000000;

pub fn global_user_page_table() -> &'static mut [PageTable; 2] {
    user_page_table_array_mut()
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct EntryFlags: u32 {
        const PRESENT = (1 << 0);
        const WRITE = (1 << 1);
        const USER = (1 << 2);
        const LARGE = (1 << 7);
    }
}

const ENTRY_FRAME_MASK: u32 = 0xfffff000;
const ENTRY_FLAGS_MASK: u32 = 0x00000fff;
const KERNEL_PAGE_DIRECTORY_INDEX: usize = KERNEL_SPACE_START_ADDRESS / SIZE_PER_PAGE_TABLE_MAP;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageDirectoryEntry(u32);

impl PageDirectoryEntry {
    pub fn set(&mut self, pfn: Option<PFN>, flags: EntryFlags) {
        let pfn = pfn.map(|pfn| usize::from(pfn)).unwrap_or(0) as u32;
        let flags = flags.bits();

        self.0 = (pfn << 12) | flags;
    }

    pub fn get(&self) -> (PFN, EntryFlags) {
        let pfn = PFN::from_val(((self.0 & ENTRY_FRAME_MASK) >> 12) as usize);
        let flags = self.0 & ENTRY_FLAGS_MASK;

        (pfn, EntryFlags::from_bits_retain(flags))
    }
}

#[repr(C)]
pub struct PageDirectory {
    entries: [PageDirectoryEntry; ENTRY_COUNT_PER_PAGE_TABLE],
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u32);

impl PageTableEntry {
    pub fn set(&mut self, pfn: Option<PFN>, flags: EntryFlags) {
        let pfn = pfn.map(|pfn| usize::from(pfn)).unwrap_or(0) as u32;
        let flags = flags.bits();

        self.0 = (pfn << 12) | flags;
    }

    pub fn get(&self) -> (PFN, EntryFlags) {
        let pfn = PFN::from_val(((self.0 & ENTRY_FRAME_MASK) >> 12) as usize);
        let flags = self.0 & ENTRY_FLAGS_MASK;

        (pfn, EntryFlags::from_bits_retain(flags))
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT_PER_PAGE_TABLE],
}

impl PageTable {
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

#[no_mangle]
pub extern "C" fn _init_page_directory() {
    let page_directory = page_directory_mut();

    page_directory.entries[KERNEL_PAGE_DIRECTORY_INDEX].set(
        Some(PFN::from(KERNEL_PAGE_TABLE_BASE_ADDRESS >> 12)),
        EntryFlags::PRESENT | EntryFlags::WRITE,
    );

    let kernel_page_table = kernel_page_table_mut();
    let mut pfn = PFN::from_val(0);
    for pte in kernel_page_table.iter_mut() {
        pte.set(Some(pfn), EntryFlags::PRESENT | EntryFlags::WRITE);
        pfn = pfn + 1;
    }
}

#[no_mangle]
pub extern "C" fn _init_user_page_table() {
    let page_directory = page_directory_mut();
    let page_table_base = USER_PAGE_TABLE_BASE_ADDRESS >> 12;

    page_directory.entries[0].set(
        Some(PFN::from_val(page_table_base)),
        EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::USER
    );

    page_directory.entries[1].set(
        Some(PFN::from_val(page_table_base + 1)),
        EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::USER
    );

    let user_ptes = user_page_table_array_mut()
        .iter_mut().map(|pt| pt.iter_mut()).flatten();

    let mut pfn = PFN::from_val(0);
    for pte in user_ptes {
        pte.set(Some(pfn), EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::USER);
        pfn = pfn + 1;
    }
}

#[no_mangle]
pub extern "C" fn _init_vesa_memory_map(
    video_memory_address: usize,
    virtual_memory_address: usize,
    video_memory_size: usize,
) {
    let vmem_start = align_down(video_memory_address, SIZE_PER_PAGE_TABLE_MAP);
    let vmem_end = align_up(
        video_memory_address + video_memory_size,
        SIZE_PER_PAGE_TABLE_MAP,
    );
    let vmem_pmds = (vmem_end - vmem_start) / SIZE_PER_PAGE_TABLE_MAP;
    let vmem_off = video_memory_address - vmem_start;
    let virt_begin = virtual_memory_address - vmem_off;

    if virt_begin % SIZE_PER_PAGE_TABLE_MAP != 0 {
        return;
    }


    let page_directory = page_directory_mut();
    let idx_begin = virt_begin / SIZE_PER_PAGE_TABLE_MAP;

    for idx in idx_begin..(idx_begin + vmem_pmds) {
        let off = idx - idx_begin;
        let vmem_pfn = PFN::from_val((vmem_start + off * SIZE_PER_PAGE_TABLE_MAP) >> 12);

        page_directory.entries[idx].set(
            Some(vmem_pfn),
            EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::LARGE,
        );
    }

    compat_flush_page_directory();
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

pub fn switch_user_struct(proc: &Process) {
    let pfn = PFN::from(PAddr::from_val(proc.addr));
    kernel_page_table_mut()[1023].set(Some(pfn), EntryFlags::PRESENT | EntryFlags::WRITE);
}

fn page_directory_mut() -> &'static mut PageDirectory {
    unsafe {
        &mut *phys_to_virt(PAddr::from(PAGE_DIRECTORY_BASE_ADDRESS)).cast()
    }
}

fn kernel_page_table_mut() -> &'static mut PageTable {
    unsafe {
        &mut *phys_to_virt(PAddr::from(KERNEL_PAGE_TABLE_BASE_ADDRESS)).cast()
    }
}

fn user_page_table_array_mut() -> &'static mut [PageTable; USER_PAGE_TABLE_COUNT] {
    unsafe {
        &mut *phys_to_virt(PAddr::from(USER_PAGE_TABLE_BASE_ADDRESS)).cast()
    }
}

fn align_down(value: usize, alignment: usize) -> usize {
    value / alignment * alignment
}

fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) / alignment * alignment
}
