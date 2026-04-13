use core::{alloc::GlobalAlloc, ptr::NonNull};

use eonix_mm::address::{Addr, PAddr};
use eonix_spin::NoContext;
use eonix_sync_base::LazyLock;
use slab_allocator::{SlabAlloc, SlabPage, SlabPageAlloc, SlabSlot};

use super::{PageList, PhysPage};
use crate::{
    mm::page_manager::{alloc_kernel_page, free_page, KERNEL_PAGE_MANAGER},
    println_info,
    sync::SpinExt as _,
};

struct Allocator;
struct SlabPageAllocImpl;

static SLAB_ALLOCATOR: LazyLock<SlabAlloc<SlabPageAllocImpl, 10>> =
    LazyLock::new(|| SlabAlloc::new_in(SlabPageAllocImpl));

const VIRT_OFFSET: usize = 0xC0000000;

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
            free_page(paddr.addr(), size, false);
        }
    }
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

#[repr(C)]
pub struct MapNode {
    pub(crate) m_size: usize,
    pub(crate) m_address_idx: usize,
}

#[no_mangle]
pub extern "C" fn mm_allocator_alloc(map: *mut MapNode, size: usize) -> usize {
    println_info!("alloc");

    if map.is_null() {
        return 0;
    }

    unsafe {
        let mut p = map;
        while (*p).m_size != 0 {
            if (*p).m_size >= size {
                let ret_idx = (*p).m_address_idx;
                (*p).m_address_idx += size;
                (*p).m_size -= size;

                if (*p).m_size == 0 {
                    let mut cur = p;
                    let mut next = p.add(1);
                    while (*next).m_size != 0 {
                        (*cur).m_address_idx = (*next).m_address_idx;
                        (*cur).m_size = (*next).m_size;
                        cur = cur.add(1);
                        next = next.add(1);
                    }
                    (*cur).m_address_idx = 0;
                    (*cur).m_size = 0;
                }

                return ret_idx;
            }
            p = p.add(1);
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn mm_allocator_free(map: *mut MapNode, size: usize, addr_idx: usize) -> usize {
    if map.is_null() {
        return 0;
    }

    unsafe {
        let mut p = map;
        while (*p).m_address_idx <= addr_idx && (*p).m_size != 0 {
            p = p.add(1);
        }

        let mut merged_prev = false;
        if p > map {
            let last = p.wrapping_sub(1);
            if addr_idx == (*last).m_address_idx + (*last).m_size {
                (*last).m_size += size;
                merged_prev = true;

                if addr_idx + size == (*p).m_address_idx {
                    (*last).m_size += (*p).m_size;
                    let mut dst = last.add(1);
                    let mut src = p.add(1);
                    while (*src).m_size != 0 {
                        (*dst).m_address_idx = (*src).m_address_idx;
                        (*dst).m_size = (*src).m_size;
                        dst = dst.add(1);
                        src = src.add(1);
                    }
                    (*dst).m_address_idx = 0;
                    (*dst).m_size = 0;
                }
            }
        }

        if !merged_prev {
            if addr_idx + size == (*p).m_address_idx && (*p).m_size != 0 {
                (*p).m_address_idx = addr_idx;
                (*p).m_size += size;
            } else if size != 0 {
                let mut tmp1 = MapNode {
                    m_size: size,
                    m_address_idx: addr_idx,
                };

                while (*p).m_size != 0 {
                    let tmp2 = MapNode {
                        m_size: (*p).m_size,
                        m_address_idx: (*p).m_address_idx,
                    };
                    (*p).m_address_idx = tmp1.m_address_idx;
                    (*p).m_size = tmp1.m_size;
                    tmp1 = tmp2;
                    p = p.add(1);
                }

                (*p).m_address_idx = tmp1.m_address_idx;
                (*p).m_size = tmp1.m_size;
            }
        }
    }

    0
}

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
