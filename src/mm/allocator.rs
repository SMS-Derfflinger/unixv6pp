use core::{alloc::GlobalAlloc, ptr::NonNull};

use eonix_mm::address::{Addr, PAddr};
use eonix_spin::NoContext;
use eonix_sync_base::LazyLock;
use slab_allocator::{SlabAlloc, SlabPageAlloc};

use super::{PageList, PhysPage};
use crate::{
    constants::platform::{KERNEL_VIRT_BASE, RAM_BASE},
    mm::page_manager::{alloc_kernel_page, free_page, KERNEL_PAGE_MANAGER},
    println_info,
    sync::SpinExt as _,
};

struct Allocator;
struct SlabPageAllocImpl;

static SLAB_ALLOCATOR: LazyLock<SlabAlloc<SlabPageAllocImpl, 10>> =
    LazyLock::new(|| SlabAlloc::new_in(SlabPageAllocImpl));

const VIRT_OFFSET: usize = KERNEL_VIRT_BASE - RAM_BASE;

pub fn phys_to_virt(paddr: PAddr) -> *mut u8 {
    let addr = paddr.addr() + VIRT_OFFSET;

    addr as *mut u8
}

pub fn virt_to_phys(ptr: *mut u8) -> PAddr {
    PAddr::from_val(ptr.addr() - VIRT_OFFSET)
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let size = layout.size().next_power_of_two();

        if size <= 2048 {
            SLAB_ALLOCATOR.alloc::<NoContext>(size).as_ptr()
        } else {
            let page = alloc_kernel_page(size);

            phys_to_virt(page.phys())
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let size = layout.size().next_power_of_two();

        if size <= 2048 {
            let ptr = unsafe {
                // SAFETY: pointers from allocators are always non-null.
                NonNull::new_unchecked(ptr)
            };

            SLAB_ALLOCATOR.dealloc::<NoContext>(ptr, size);
        } else {
            let paddr = virt_to_phys(ptr);
            free_page(paddr.addr(), size);
        }
    }
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

unsafe impl SlabPageAlloc for SlabPageAllocImpl {
    type Page = PhysPage;
    type PageList = PageList;

    fn alloc_slab_page(&self) -> &'static mut Self::Page {
        let page = KERNEL_PAGE_MANAGER
            .lock()
            .alloc_order(0)
            .expect("Out of memory");

        unsafe {
            page.slab_init();
        }

        page
    }
}
