#ifndef KERNEL_ALLOCATOR
#define KERNEL_ALLOCATOR

#include "MapNode.h"
#include "Allocator.h"

#ifdef __cplusplus
extern "C" {
#endif

unsigned long mm_kernel_allocator_initialize(
    MapNode map[],
    unsigned long map_len,
    unsigned long start_idx,
    unsigned long size
);
unsigned long mm_kernel_allocator_alloc(MapNode map[], unsigned long size);
unsigned long mm_kernel_allocator_free(
    MapNode map[],
    unsigned long size,
    unsigned long memory_start_address
);

#ifdef __cplusplus
}
#endif

class KernelAllocator
{
public:
    static const unsigned int MEMORY_MAP_ARRAY_SIZE = 0x200;
    static const unsigned int KERNEL_HEAP_START_ADDR = 0x180000 + 0xC0000000;
    static const unsigned int KERNEL_HEAP_SIZE = 0x80000;

public:
    KernelAllocator(Allocator* allocator)
    {
        (void)allocator;
    }

    ~KernelAllocator() = default;

    int Initialize()
    {
        return static_cast<int>(mm_kernel_allocator_initialize(
            this->map,
            MEMORY_MAP_ARRAY_SIZE,
            KERNEL_HEAP_START_ADDR,
            KERNEL_HEAP_SIZE
        ));
    }

    unsigned long AllocMemory(unsigned long size)
    {
        return mm_kernel_allocator_alloc(this->map, size);
    }

    unsigned long FreeMemeory(unsigned long size, unsigned long memoryStartAddress)
    {
        return mm_kernel_allocator_free(this->map, size, memoryStartAddress);
    }

public:
    MapNode map[KernelAllocator::MEMORY_MAP_ARRAY_SIZE];
};

#endif
