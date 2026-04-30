use core::ptr;

use eonix_mm::address::PAddr;

mod allocator;
mod kstack;
mod page;
mod page_manager;
mod swapper_manager;
mod zone;

pub use allocator::{phys_to_virt, virt_to_phys};
pub use kstack::KernelStack;
pub use page::{KernelPages, PageList, PhysPage, UserPages, PAGE_SIZE};
pub use page_manager::{free_page, init_page_managers, KERNEL_PAGE_MANAGER, USER_PAGE_MANAGER};
pub use swapper_manager::{swap_alloc, swap_free, SWAPPER_AREAS};
pub use zone::ZONE;

/// Copy bytes between pseudo-physical addresses through the kernel linear map.
pub fn phys_copy(from: PAddr, to: PAddr, len: usize) {
    if len == 0 || from == to {
        return;
    }

    let src_ptr = phys_to_virt(from) as *const u8;
    let dst_ptr = phys_to_virt(to);

    unsafe {
        ptr::copy(src_ptr, dst_ptr, len);
    }
}
