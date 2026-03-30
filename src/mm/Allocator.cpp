#include "Allocator.h"
#include "mm_rust_ffi.h"

Allocator Allocator::m_Instance;

Allocator &Allocator::GetInstance() {
    return Allocator::m_Instance;
}

unsigned long Allocator::Alloc(MapNode map[], unsigned long size) {
    return mm_allocator_alloc(map, size);
}

unsigned long Allocator::Free(MapNode map[], unsigned long size, unsigned long addrIdx) {
    return mm_allocator_free(map, size, addrIdx);
}
