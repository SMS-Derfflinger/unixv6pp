use core::{mem::MaybeUninit, ptr::NonNull};

use buddy_allocator::{BuddyAllocator, BuddyFolio};
use eonix_mm::{
    address::{Addr, PAddr, PRange},
    paging::{FolioList, FolioListSized, Zone, PFN},
};
use eonix_spin::{NoContext, Spin, SpinGuard};
use eonix_sync_base::{LazyLock, Relax};
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListAtomicLink, UnsafeRef};
use slab_allocator::{SlabAlloc, SlabPage, SlabPageAlloc, SlabSlot};

use crate::println_trace;

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

static mut PAGES: MaybeUninit<[PhysPage; PAGE_COUNT]> = MaybeUninit::zeroed();

fn page_array() -> &'static [PhysPage; PAGE_COUNT] {
    unsafe { PAGES.assume_init_mut() }
}

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

impl PhysPage {
    pub fn phys(&self) -> PAddr {
        PAddr::from(self.pfn())
    }
}

impl BuddyFolio for PhysPage {
    fn pfn(&self) -> PFN {
        let ptr = self as *const Self;

        let offset = unsafe { ptr.offset_from(page_array().as_ptr()) };

        match unsafe { ptr.offset_from(page_array().as_ptr()) } {
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
        self.is_buddy = value;
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
        if usize::from(pfn) >= PAGE_COUNT {
            return None;
        }

        Some(NonNull::from_ref(&page_array()[usize::from(pfn)]))
    }
}

static KERNEL_PAGE_MANAGER: Spin<BuddyAllocator<MemoryZone, PageList>> =
    Spin::new(BuddyAllocator::new(&MemoryZone));

static USER_PAGE_MANAGER: Spin<BuddyAllocator<MemoryZone, PageList>> =
    Spin::new(BuddyAllocator::new(&MemoryZone));

static SLAB_ALLOCATOR: LazyLock<SlabAlloc<SlabPageAllocImpl, 10>> =
    LazyLock::new(|| SlabAlloc::new_in(SlabPageAllocImpl));

struct SlabPageAllocImpl;

unsafe impl SlabPageAlloc for SlabPageAllocImpl {
    type Page = PhysPage;
    type PageList = PageList;

    fn alloc_slab_page(&self) -> &'static mut Self::Page {
        KERNEL_PAGE_MANAGER
            .lock()
            .alloc_order(0)
            .expect("Out of memory")
    }
}

impl SlabPage for PhysPage {
    fn get_data_ptr(&self) -> NonNull<[u8]> {
        todo!()
    }

    fn get_free_slot(&self) -> Option<NonNull<SlabSlot>> {
        todo!()
    }

    fn set_free_slot(&mut self, next: Option<NonNull<SlabSlot>>) {
        todo!()
    }

    fn get_alloc_count(&self) -> usize {
        todo!()
    }

    fn inc_alloc_count(&mut self) -> usize {
        todo!()
    }

    fn dec_alloc_count(&mut self) -> usize {
        todo!()
    }

    unsafe fn from_allocated(ptr: NonNull<u8>) -> &'static mut Self {
        todo!()
    }
}

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

#[no_mangle]
pub extern "C" fn init_page_managers() {
    let krange = PRange::new(PAddr::from_val(0x204000), PAddr::from_val(0x400000));
    let urange = PRange::new(PAddr::from_val(0x400000), PAddr::from_val(MEM_SIZE));

    unsafe {
        for page in PAGES.assume_init_mut().iter_mut() {
            *page = PhysPage::new();
        }
    }

    {
        let mut kpm = KERNEL_PAGE_MANAGER.lock();
        kpm.create_folios(krange.start(), krange.end());
    }

    {
        let mut upm = USER_PAGE_MANAGER.lock();
        upm.create_folios(urange.start(), urange.end());
    }
}

/// Allocate a page and return its physical address
#[no_mangle]
pub extern "C" fn alloc_page(size: usize, user: bool) -> usize {
    let mut allocator = if user {
        USER_PAGE_MANAGER.lock()
    } else {
        KERNEL_PAGE_MANAGER.lock()
    };

    let aligned_size = size.next_power_of_two();
    let order = aligned_size.trailing_zeros() - 12;

    let page = allocator.alloc_order(order).expect("Out of memory");

    #[cfg(trace_alloc)]
    println_trace!("Allocated {:?} size={:#x}", page.pfn(), size);

    page.phys().addr()
}

#[no_mangle]
pub extern "C" fn free_page(addr: usize, size: usize, user: bool) {
    if size == 0 {
        return;
    }

    let paddr = PAddr::from_val(addr);

    let mut allocator = if user {
        USER_PAGE_MANAGER.lock()
    } else {
        KERNEL_PAGE_MANAGER.lock()
    };

    let mut page_ptr = MemoryZone.get_page(PFN::from(paddr)).expect("Bad address");
    let page = unsafe {
        // SAFETY: We've got the page pointer from MemoryZone.
        page_ptr.as_mut()
    };

    assert!(size <= (1 << (page.order as usize + 12)) ,"Wrong size");

    #[cfg(trace_alloc)]
    println_trace!("Deallocate {:?} size={:#x}", page.pfn(), size);

    unsafe {
        allocator.dealloc(page);
    }
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
