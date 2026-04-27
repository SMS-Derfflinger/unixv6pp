use eonix_mm::{address::{Addr, PAddr}, paging::PFN};

use crate::{
    machine::{EntryFlags, flush_tlb, kernel_page_table_mut},
    sync::IrqGuard,
};

mod allocator;
mod kstack;
mod page;
mod page_manager;
mod swapper_manager;
mod zone;

pub use allocator::{phys_to_virt, virt_to_phys};
pub use kstack::KernelStack;
pub use page::{KernelPages, PageList, PhysPage, UserPages, PAGE_SIZE};
pub use page_manager::{
    free_page, init_page_managers, KERNEL_PAGE_MANAGER, USER_PAGE_MANAGER,
};
pub use swapper_manager::SWAPPER_AREAS;
pub use zone::ZONE;

/// Temporarily borrow two kernel PTEs to copy bytes between physical pages.
pub fn phys_copy(from: PAddr, to: PAddr, len: usize) {
    const BORROWED_PTE: usize = 256;
    const BORROW_WINDOW_BASE: usize = 0xc020_0000;

    let _ctx = IrqGuard::disable_save();
    let kernel_pt = kernel_page_table_mut();

    let original_src = kernel_pt[BORROWED_PTE].get();
    let original_dst = kernel_pt[BORROWED_PTE + 1].get();
    let flags = EntryFlags::VALID
        | EntryFlags::READ
        | EntryFlags::WRITE
        | EntryFlags::ACCESSED
        | EntryFlags::DIRTY;

    for offset in 0..len {
        let src = from.addr() + offset;
        let dst = to.addr() + offset;

        kernel_pt[BORROWED_PTE].set(Some(PFN::from_val(src / PAGE_SIZE)), flags);
        kernel_pt[BORROWED_PTE + 1].set(Some(PFN::from_val(dst / PAGE_SIZE)), flags);
        flush_tlb();

        let src_ptr = (BORROW_WINDOW_BASE + BORROWED_PTE * PAGE_SIZE + src % PAGE_SIZE) as *const u8;
        let dst_ptr =
            (BORROW_WINDOW_BASE + (BORROWED_PTE + 1) * PAGE_SIZE + dst % PAGE_SIZE) as *mut u8;

        unsafe {
            dst_ptr.write_volatile(src_ptr.read_volatile());
        }
    }

    kernel_pt[BORROWED_PTE].set(
        original_src
            .1
            .contains(EntryFlags::VALID)
            .then_some(original_src.0),
        original_src.1,
    );
    kernel_pt[BORROWED_PTE + 1].set(
        original_dst
            .1
            .contains(EntryFlags::VALID)
            .then_some(original_dst.0),
        original_dst.1,
    );
    flush_tlb();
}
