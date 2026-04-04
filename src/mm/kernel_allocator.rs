use super::allocator::{mm_allocator_alloc, mm_allocator_free, MapNode};

#[no_mangle]
pub extern "C" fn mm_kernel_allocator_initialize(
    map: *mut MapNode,
    map_len: usize,
    start_idx: usize,
    size: usize,
) -> usize {
    if map.is_null() || map_len == 0 {
        return 1;
    }

    unsafe {
        for i in 0..map_len {
            let node = map.add(i);
            (*node).m_address_idx = 0;
            (*node).m_size = 0;
        }

        (*map).m_address_idx = start_idx;
        (*map).m_size = size;
    }

    0
}

#[no_mangle]
pub extern "C" fn mm_kernel_allocator_alloc(map: *mut MapNode, size: usize) -> usize {
    mm_allocator_alloc(map, size)
}

#[no_mangle]
pub extern "C" fn mm_kernel_allocator_free(
    map: *mut MapNode,
    size: usize,
    memory_start_address: usize,
) -> usize {
    mm_allocator_free(map, size, memory_start_address)
}
