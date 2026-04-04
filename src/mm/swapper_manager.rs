use super::allocator::{mm_allocator_alloc, mm_allocator_free, MapNode};

#[no_mangle]
pub extern "C" fn mm_swapper_manager_initialize(
    map: *mut MapNode,
    map_len: usize,
    zone_start_block: usize,
    zone_size: usize,
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

        (*map).m_address_idx = zone_start_block;
        (*map).m_size = zone_size;
    }

    0
}

#[no_mangle]
pub extern "C" fn mm_swapper_manager_alloc(
    map: *mut MapNode,
    size: usize,
    block_size: usize,
) -> usize {
    if block_size == 0 {
        return 0;
    }

    mm_allocator_alloc(map, (size + (block_size - 1)) / block_size)
}

#[no_mangle]
pub extern "C" fn mm_swapper_manager_free(
    map: *mut MapNode,
    size: usize,
    start_block: usize,
    block_size: usize,
) -> usize {
    if block_size == 0 {
        return 0;
    }

    mm_allocator_free(map, (size + (block_size - 1)) / block_size, start_block)
}
