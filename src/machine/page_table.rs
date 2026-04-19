use core::arch::asm;

const ENTRY_COUNT_PER_PAGE_TABLE: usize = 1024;
const SIZE_PER_PAGE_TABLE_MAP: usize = 0x400000;
const PAGE_DIRECTORY_BASE_ADDRESS: usize = 0x200000;
const KERNEL_PAGE_TABLE_BASE_ADDRESS: usize = 0x201000;
const USER_PAGE_TABLE_BASE_ADDRESS: usize = 0x202000;
const USER_PAGE_TABLE_COUNT: usize = 2;
const KERNEL_SPACE_START_ADDRESS: usize = 0xc0000000;

const PRESENT: EntryFlags = EntryFlags(1 << 0);
const READ_WRITE: EntryFlags = EntryFlags(1 << 1);
const USER_ACCESSIBLE: EntryFlags = EntryFlags(1 << 2);
const PAGE_SIZE: EntryFlags = EntryFlags(1 << 7);

const ENTRY_FRAME_MASK: u32 = 0xfffff000;
const ENTRY_FLAGS_MASK: u32 = 0x00000fff;
const KERNEL_PAGE_DIRECTORY_INDEX: usize = KERNEL_SPACE_START_ADDRESS / SIZE_PER_PAGE_TABLE_MAP;

#[derive(Clone, Copy)]
struct EntryFlags(u32);

impl EntryFlags {
    const fn empty() -> Self {
        Self(0)
    }

    const fn bits(self) -> u32 {
        self.0
    }

    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Clone, Copy)]
enum Privilege {
    Kernel,
    User,
}

impl Privilege {
    const fn flags(self) -> EntryFlags {
        match self {
            Self::Kernel => EntryFlags::empty(),
            Self::User => USER_ACCESSIBLE,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageDirectoryEntry(u32);

impl PageDirectoryEntry {
    fn page_table(page_table_base: usize, privilege: Privilege) -> Self {
        Self::new(
            page_table_base,
            PRESENT.union(READ_WRITE).union(privilege.flags()),
        )
    }

    fn large_page(page_base: usize, privilege: Privilege) -> Self {
        Self::new(
            page_base,
            PRESENT
                .union(READ_WRITE)
                .union(PAGE_SIZE)
                .union(privilege.flags()),
        )
    }

    fn new(frame: usize, flags: EntryFlags) -> Self {
        Self((((frame as u32) << 12) & ENTRY_FRAME_MASK) | (flags.bits() & ENTRY_FLAGS_MASK))
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
    fn page(page_base: usize, privilege: Privilege) -> Self {
        Self::new(
            page_base,
            PRESENT.union(READ_WRITE).union(privilege.flags()),
        )
    }

    fn new(frame: usize, flags: EntryFlags) -> Self {
        Self((((frame as u32) << 12) & ENTRY_FRAME_MASK) | (flags.bits() & ENTRY_FLAGS_MASK))
    }
}

#[repr(C)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT_PER_PAGE_TABLE],
}

#[no_mangle]
pub extern "C" fn _init_page_directory() {
    let page_directory = page_directory_mut();
    page_directory.entries[KERNEL_PAGE_DIRECTORY_INDEX] =
        PageDirectoryEntry::page_table(KERNEL_PAGE_TABLE_BASE_ADDRESS >> 12, Privilege::Kernel);

    let kernel_page_table = kernel_page_table_mut();
    for (index, entry) in kernel_page_table.entries.iter_mut().enumerate() {
        *entry = PageTableEntry::page(index, Privilege::Kernel);
    }
}

#[no_mangle]
pub extern "C" fn _init_user_page_table() {
    let page_directory = page_directory_mut();
    let user_page_tables = user_page_table_array_mut();
    let mut page_table_base = USER_PAGE_TABLE_BASE_ADDRESS >> 12;

    for table_index in 0..USER_PAGE_TABLE_COUNT {
        page_directory.entries[table_index] =
            PageDirectoryEntry::page_table(page_table_base, Privilege::User);
        page_table_base += 1;

        for (entry_index, entry) in user_page_tables[table_index].entries.iter_mut().enumerate() {
            *entry = PageTableEntry::page(
                entry_index + table_index * ENTRY_COUNT_PER_PAGE_TABLE,
                Privilege::User,
            );
        }
    }
}

#[no_mangle]
pub extern "C" fn _init_vesa_memory_map(
    video_memory_address: usize,
    virtual_memory_address: usize,
    video_memory_size: usize,
) {
    let video_memory_begin = align_down(video_memory_address, SIZE_PER_PAGE_TABLE_MAP);
    let virtual_memory_begin = virtual_memory_address - (video_memory_address - video_memory_begin);
    if virtual_memory_begin % SIZE_PER_PAGE_TABLE_MAP != 0 {
        return;
    }

    let video_memory_end = align_up(
        video_memory_address + video_memory_size,
        SIZE_PER_PAGE_TABLE_MAP,
    );

    let page_directory = page_directory_mut();
    let mut address = video_memory_begin;
    while address < video_memory_end {
        let virtual_address = address + virtual_memory_begin - video_memory_begin;
        page_directory.entries[virtual_address / SIZE_PER_PAGE_TABLE_MAP] =
            PageDirectoryEntry::large_page(address >> 12, Privilege::Kernel);

        address += SIZE_PER_PAGE_TABLE_MAP;
    }

    flush_page_directory();
}

#[no_mangle]
pub extern "C" fn _page_directory() -> *mut PageDirectory {
    page_directory_ptr()
}

#[no_mangle]
pub extern "C" fn _kernel_page_table() -> *mut PageTable {
    kernel_page_table_ptr()
}

#[no_mangle]
pub extern "C" fn _user_page_table_array() -> *mut PageTable {
    user_page_table_array_ptr()
}

fn page_directory_mut() -> &'static mut PageDirectory {
    unsafe { &mut *page_directory_ptr() }
}

fn kernel_page_table_mut() -> &'static mut PageTable {
    unsafe { &mut *kernel_page_table_ptr() }
}

fn user_page_table_array_mut() -> &'static mut [PageTable; USER_PAGE_TABLE_COUNT] {
    unsafe { &mut *(user_page_table_array_ptr() as *mut [PageTable; USER_PAGE_TABLE_COUNT]) }
}

fn page_directory_ptr() -> *mut PageDirectory {
    (PAGE_DIRECTORY_BASE_ADDRESS + KERNEL_SPACE_START_ADDRESS) as *mut PageDirectory
}

fn kernel_page_table_ptr() -> *mut PageTable {
    (KERNEL_PAGE_TABLE_BASE_ADDRESS + KERNEL_SPACE_START_ADDRESS) as *mut PageTable
}

fn user_page_table_array_ptr() -> *mut PageTable {
    (USER_PAGE_TABLE_BASE_ADDRESS + KERNEL_SPACE_START_ADDRESS) as *mut PageTable
}

fn align_down(value: usize, alignment: usize) -> usize {
    value / alignment * alignment
}

fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) / alignment * alignment
}

fn flush_page_directory() {
    unsafe {
        asm!(
            "mov cr3, eax",
            in("eax") PAGE_DIRECTORY_BASE_ADDRESS,
            options(nostack, preserves_flags),
        );
    }
}
