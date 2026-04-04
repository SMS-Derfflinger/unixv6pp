#[repr(C)]
pub struct MapNode {
    m_size: usize,
    m_address_idx: usize,
}

#[no_mangle]
pub extern "C" fn mm_allocator_alloc(map: *mut MapNode, size: usize) -> usize {
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
