#ifndef ALLOCATOR_H
#define ALLOCATOR_H

#include "MapNode.h"

#ifdef __cplusplus
extern "C" {
#endif

unsigned long mm_allocator_alloc(MapNode map[], unsigned long size);
unsigned long mm_allocator_free(MapNode map[], unsigned long size, unsigned long addr_idx);

#ifdef __cplusplus
}
#endif

class Allocator
{
public:
    unsigned long Alloc(MapNode map[], unsigned long size)
    {
        return mm_allocator_alloc(map, size);
    }

    unsigned long Free(MapNode map[], unsigned long size, unsigned long addrIdx)
    {
        return mm_allocator_free(map, size, addrIdx);
    }

    static Allocator& GetInstance()
    {
        static Allocator instance;
        return instance;
    }
};

#endif
