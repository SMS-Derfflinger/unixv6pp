use core::ptr::NonNull;

use buddy_allocator::{BuddyAllocator, BuddyFolio};
use eonix_mm::{
    address::{Addr, PAddr, PRange},
    paging::{FolioList, FolioListSized, Zone, PFN},
};
use eonix_spin::{NoContext, Spin, SpinGuard};
use eonix_sync_base::Relax;
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListAtomicLink, UnsafeRef};

use super::allocator::{mm_allocator_alloc, mm_allocator_free, MapNode};

const PAGE_SIZE: usize = 0x1000;
const MEM_SIZE: usize = 64 * 1024 * 1024;

const PAGE_COUNT: usize = MEM_SIZE / PAGE_SIZE;
const PAGE_COUNTI: isize = PAGE_COUNT as isize;

struct PhysPage {
    link: LinkedListAtomicLink,
    order: u8,
    is_slab: bool,
    is_buddy: bool,
}

struct MemoryZone;

intrusive_adapter!(PagesAdapter = UnsafeRef<PhysPage>: PhysPage { link: LinkedListAtomicLink });
struct PageList(LinkedList<PagesAdapter>);

static PAGES: [PhysPage; PAGE_COUNT] = [const { PhysPage::new() }; PAGE_COUNT];

impl PhysPage {
    pub const fn new() -> Self {
        Self {
            link: LinkedListAtomicLink::new(),
            order: 0,
            is_slab: false,
            is_buddy: false,
        }
    }
}

impl BuddyFolio for PhysPage {
    fn pfn(&self) -> PFN {
        let ptr = self as *const Self;

        let offset = unsafe { ptr.offset_from(PAGES.as_ptr()) };

        match unsafe { ptr.offset_from(PAGES.as_ptr()) } {
            ..0 | PAGE_COUNTI.. => panic!("Overflow"),
            offset => PFN::from_val(offset as usize),
        }
    }

    fn get_order(&self) -> u32 {
        self.order as u32
    }

    fn is_buddy(&self) -> bool {
        self.is_buddy
    }

    fn set_order(&mut self, order: u32) {
        self.order = order as u8;
    }

    fn set_buddy(&mut self, value: bool) {
        self.is_buddy = true;
    }
}

impl FolioList for PageList {
    type Folio = PhysPage;

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn peek_head(&mut self) -> Option<&mut Self::Folio> {
        let front = self.0.front().clone_pointer()?;

        unsafe { UnsafeRef::into_raw(front).as_mut() }
    }

    fn pop_head(&mut self) -> Option<&'static mut Self::Folio> {
        let front = self.0.front_mut().remove()?;

        unsafe { UnsafeRef::into_raw(front).as_mut() }
    }

    fn push_tail(&mut self, page: &'static mut Self::Folio) {
        let page = unsafe { UnsafeRef::from_raw(page as *const _) };

        self.0.push_back(page);
    }

    fn remove(&mut self, page: &mut Self::Folio) {
        let mut cursor = unsafe { self.0.cursor_mut_from_ptr(page as *const _) };
        cursor.remove();
    }
}

impl FolioListSized for PageList {
    const NEW: Self = PageList(LinkedList::new(PagesAdapter::NEW));
}

impl Zone for MemoryZone {
    type Page = PhysPage;

    fn contains_prange(&self, range: PRange) -> bool {
        let start = range.start();
        let end = range.end();

        range.end() <= PAddr::from_val(MEM_SIZE)
    }

    fn get_page(&self, pfn: PFN) -> Option<NonNull<Self::Page>> {
        if PAddr::from(pfn).addr() >= MEM_SIZE {
            return None;
        }

        Some(NonNull::from_ref(&PAGES[usize::from(pfn)]))
    }
}

static KERNEL_PAGE_MANAGER: Spin<BuddyAllocator<MemoryZone, PageList>> =
    Spin::new(BuddyAllocator::new(&MemoryZone));

static USER_PAGE_MANAGER: Spin<BuddyAllocator<MemoryZone, PageList>> =
    Spin::new(BuddyAllocator::new(&MemoryZone));

pub trait SpinExt<T, R> {
    fn lock(&self) -> SpinGuard<'_, T, NoContext, R>;
}

impl<T, R> SpinExt<T, R> for Spin<T, R>
where
    R: Relax,
{
    fn lock(&self) -> SpinGuard<'_, T, NoContext, R> {
        self.lock_ctx::<NoContext>()
    }
}

pub extern "C" fn init_page_managers() {
    let krange = PRange::new(PAddr::from_val(0x204000), PAddr::from_val(0x400000));
    let urange = PRange::new(PAddr::from_val(0x400000), PAddr::from_val(MEM_SIZE));

    {
        let mut kpm = KERNEL_PAGE_MANAGER.lock();
        kpm.create_folios(krange.start(), krange.end());
    }

    {
        let mut upm = USER_PAGE_MANAGER.lock();
        upm.create_folios(urange.start(), urange.end());
    }
}

#[no_mangle]
pub extern "C" fn mm_page_manager_initialize(map: *mut MapNode, map_len: usize) -> usize {
    if map.is_null() || map_len == 0 {
        return 1;
    }

    unsafe {
        for i in 0..map_len {
            let node = map.add(i);
            (*node).m_address_idx = 0;
            (*node).m_size = 0;
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn mm_page_manager_init_pool(
    map: *mut MapNode,
    map_len: usize,
    page_size: usize,
    pool_start_addr: usize,
    pool_size: usize,
) -> usize {
    if mm_page_manager_initialize(map, map_len) != 0 || page_size == 0 {
        return 1;
    }

    unsafe {
        (*map).m_address_idx = pool_start_addr / page_size;
        (*map).m_size = pool_size / page_size;
    }

    0
}

#[no_mangle]
pub extern "C" fn mm_page_manager_alloc(map: *mut MapNode, size: usize, page_size: usize) -> usize {
    if page_size == 0 {
        return 0;
    }

    let pages = (size + (page_size - 1)) / page_size;
    mm_allocator_alloc(map, pages) * page_size
}

#[no_mangle]
pub extern "C" fn mm_page_manager_free(
    map: *mut MapNode,
    size: usize,
    start_address: usize,
    page_size: usize,
) -> usize {
    if page_size == 0 {
        return 0;
    }

    let pages = (size + (page_size - 1)) / page_size;
    mm_allocator_free(map, pages, start_address / page_size)
}
