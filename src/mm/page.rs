use core::{mem::MaybeUninit, ops::Deref, ptr::NonNull};

use buddy_allocator::BuddyAllocator;
use eonix_mm::{
    address::PAddr,
    paging::{Folio, FolioList, FolioListSized, Zone, PFN},
};
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListAtomicLink, UnsafeRef};
use slab_allocator::{SlabPage, SlabSlot};

use crate::{
    mm::{
        allocator::{phys_to_virt, virt_to_phys},
        zone::{MemoryZone, ZONE},
        KERNEL_PAGE_MANAGER, USER_PAGE_MANAGER,
    },
    sync::{KernelSpinGuard, SpinExt},
};

pub const PAGE_SIZE: usize = 0x1000;

#[derive(Clone, Copy)]
struct SlabPageData {
    next_free: Option<NonNull<SlabSlot>>,
    alloced: usize,
}

#[derive(Clone, Copy)]
struct UserPageData {
    refcount: usize,
}

union PageData {
    slab: SlabPageData,
    user: UserPageData,
}

pub struct PhysPage {
    pub link: LinkedListAtomicLink,
    pub order: u8,
    pub is_slab: bool,
    pub is_buddy: bool,
    data: PageData,
}

pub struct Pages<const USER: bool> {
    raw_handle: Option<&'static mut PhysPage>,
}

pub type KernelPages = Pages<false>;
pub type UserPages = Pages<true>;

impl PageData {
    pub const fn uninit() -> Self {
        unsafe { MaybeUninit::uninit().assume_init() }
    }
}

intrusive_adapter!(PagesAdapter = UnsafeRef<PhysPage>: PhysPage { link: LinkedListAtomicLink });
pub struct PageList(LinkedList<PagesAdapter>);

impl PhysPage {
    pub const fn new() -> Self {
        Self {
            link: LinkedListAtomicLink::new(),
            order: 0,
            is_slab: false,
            is_buddy: false,
            data: PageData::uninit(),
        }
    }

    pub fn phys(&self) -> PAddr {
        PAddr::from(self.pfn())
    }

    pub unsafe fn slab_init(&mut self) {
        self.data = PageData {
            slab: SlabPageData {
                next_free: None,
                alloced: 0,
            },
        };
    }
}

impl<const USER: bool> Pages<USER> {
    fn allocator() -> KernelSpinGuard<'static, BuddyAllocator<MemoryZone, PageList>> {
        if USER {
            USER_PAGE_MANAGER.lock()
        } else {
            KERNEL_PAGE_MANAGER.lock()
        }
    }
}

impl UserPages {
    #[inline(always)]
    pub fn alloc_bytes(bytes: usize) -> Option<Self> {
        assert_ne!(bytes, 0, "Can't alloc 0 size");
        let aligned_size = bytes.next_power_of_two();
        let order = aligned_size.trailing_zeros() - 12;
        Self::alloc(order)
    }

    pub fn alloc(order: u32) -> Option<Self> {
        let raw = Self::allocator().alloc_order(order)?;

        Some(Self {
            raw_handle: Some(raw),
        })
    }
}

impl KernelPages {
    #[inline(always)]
    pub fn alloc_bytes(bytes: usize) -> Self {
        assert_ne!(bytes, 0, "Can't alloc 0 size");
        let aligned_size = bytes.next_power_of_two();
        let order = aligned_size.trailing_zeros() - 12;
        Self::alloc(order)
    }

    pub fn alloc(order: u32) -> Self {
        let raw = Self::allocator().alloc_order(order).expect("Out of memory");

        Self {
            raw_handle: Some(raw),
        }
    }
}

impl<const USER: bool> Deref for Pages<USER> {
    type Target = PhysPage;

    fn deref(&self) -> &Self::Target {
        self.raw_handle.as_ref().unwrap()
    }
}

impl<const USER: bool> Drop for Pages<USER> {
    fn drop(&mut self) {
        let raw = self.raw_handle.take().unwrap();
        unsafe {
            Self::allocator().dealloc(raw);
        }
    }
}

impl Folio for PhysPage {
    fn pfn(&self) -> PFN {
        ZONE.get_pfn(self)
    }

    fn order(&self) -> u32 {
        self.order as u32
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

impl SlabPage for PhysPage {
    fn get_data_ptr(&self) -> NonNull<[u8]> {
        unsafe {
            // SAFETY: We don't allocate PFN(0).
            NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                phys_to_virt(self.phys()),
                self.len(),
            ))
        }
    }

    fn get_free_slot(&self) -> Option<NonNull<SlabSlot>> {
        unsafe { self.data.slab.next_free }
    }

    fn set_free_slot(&mut self, next: Option<NonNull<SlabSlot>>) {
        self.data.slab.next_free = next;
    }

    fn get_alloc_count(&self) -> usize {
        unsafe { self.data.slab.alloced }
    }

    fn inc_alloc_count(&mut self) -> usize {
        let old_value = unsafe { self.data.slab.alloced };

        unsafe {
            self.data.slab.alloced += 1;
        }

        old_value
    }

    fn dec_alloc_count(&mut self) -> usize {
        let old_value = unsafe { self.data.slab.alloced };

        unsafe {
            self.data.slab.alloced -= 1;
        }

        old_value
    }

    unsafe fn from_allocated(ptr: NonNull<u8>) -> &'static mut Self {
        let pfn = PFN::from(virt_to_phys(ptr.as_ptr()));
        let mut page_ptr = ZONE.get_page(pfn).expect("Invalid address");

        unsafe { page_ptr.as_mut() }
    }
}
