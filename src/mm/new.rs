use core::mem;

use super::allocator::MapNode;
use super::kernel_allocator::{mm_kernel_allocator_alloc, mm_kernel_allocator_free};

const NEW_HEADER_SIZE: usize = mem::size_of::<u32>();

#[no_mangle]
pub extern "C" fn mm_new_alloc(map: *mut MapNode, size: usize) -> usize {
    if map.is_null() {
        return 0;
    }

    let total_size = size.saturating_add(NEW_HEADER_SIZE);
    let address = mm_kernel_allocator_alloc(map, total_size);
    if address == 0 {
        return 0;
    }

    unsafe {
        (address as *mut u32).write(size as u32);
    }

    address + NEW_HEADER_SIZE
}

#[no_mangle]
pub extern "C" fn mm_new_free(map: *mut MapNode, ptr: usize) {
    if map.is_null() || ptr == 0 {
        return;
    }

    let header_addr = ptr - NEW_HEADER_SIZE;
    let size = unsafe { (header_addr as *const u32).read() as usize };
    let total_size = size.saturating_add(NEW_HEADER_SIZE);

    mm_kernel_allocator_free(map, total_size, header_addr);
}
