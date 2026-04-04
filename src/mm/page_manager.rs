use super::allocator::{mm_allocator_alloc, mm_allocator_free, MapNode};

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
pub extern "C" fn mm_page_manager_alloc(
    map: *mut MapNode,
    size: usize,
    page_size: usize,
) -> usize {
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
