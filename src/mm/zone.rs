use core::{mem::MaybeUninit, ptr::NonNull};

use eonix_mm::{address::{PAddr, PRange}, paging::{PFN, Zone}};

use crate::mm::PAGE_SIZE;

use super::PhysPage;

pub const MEM_SIZE: usize = 64 * 1024 * 1024;

pub const PAGE_COUNT: usize = MEM_SIZE / PAGE_SIZE;
pub const PAGE_COUNTI: isize = PAGE_COUNT as isize;

pub const ZONE: MemoryZone = MemoryZone();

pub struct MemoryZone();

static mut PAGES: MaybeUninit<[PhysPage; PAGE_COUNT]> = MaybeUninit::zeroed();

fn page_array() -> &'static [PhysPage; PAGE_COUNT] {
    #[allow(static_mut_refs)]
    unsafe { PAGES.assume_init_mut() }
}

impl MemoryZone {
    pub fn get_pfn(&self, page: &<Self as Zone>::Page) -> PFN {
        let ptr = page as *const <Self as Zone>::Page;

        let offset = unsafe { ptr.offset_from(page_array().as_ptr()) };
        match offset {
            ..0 | PAGE_COUNTI.. => panic!("Overflow"),
            offset => PFN::from_val(offset as usize),
        }
    }
}

impl Zone for MemoryZone {
    type Page = PhysPage;

    fn contains_prange(&self, range: PRange) -> bool {
        range.end() <= PAddr::from_val(MEM_SIZE)
    }

    fn get_page(&self, pfn: PFN) -> Option<NonNull<Self::Page>> {
        if usize::from(pfn) >= PAGE_COUNT {
            return None;
        }

        Some(NonNull::from_ref(&page_array()[usize::from(pfn)]))
    }
}

pub fn init_zone() {
    #[allow(static_mut_refs)]
    unsafe {
        for page in PAGES.assume_init_mut().iter_mut() {
            *page = PhysPage::new();
        }
    }
}
