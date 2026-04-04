#ifndef SWAPPER_MANAGER_H
#define SWAPPER_MANAGER_H

#include "MapNode.h"
#include "Allocator.h"

#ifdef __cplusplus
extern "C" {
#endif

unsigned long mm_swapper_manager_initialize(
    MapNode map[],
    unsigned long map_len,
    unsigned long zone_start_block,
    unsigned long zone_size
);
unsigned long mm_swapper_manager_alloc(
    MapNode map[],
    unsigned long size,
    unsigned long block_size
);
unsigned long mm_swapper_manager_free(
    MapNode map[],
    unsigned long size,
    unsigned long start_block,
    unsigned long block_size
);

#ifdef __cplusplus
}
#endif

class SwapperManager
{
public:
    inline static unsigned int SWAPPER_ZONE_START_BLOCK = 18200;
    inline static unsigned int SWAPPER_ZONE_SIZE = 2000;

    static const unsigned int SWAPPER_MAP_ARRAY_SIZE = 0x200;
    static const unsigned int BLOCK_SIZE = 512;

public:
    SwapperManager(Allocator* pAllocator)
    {
        (void)pAllocator;
    }

    ~SwapperManager() = default;

    int Initialize()
    {
        return static_cast<int>(mm_swapper_manager_initialize(
            this->map,
            SWAPPER_MAP_ARRAY_SIZE,
            SWAPPER_ZONE_START_BLOCK,
            SWAPPER_ZONE_SIZE
        ));
    }

    int AllocSwap(unsigned long size)
    {
        return static_cast<int>(mm_swapper_manager_alloc(this->map, size, BLOCK_SIZE));
    }

    int FreeSwap(unsigned long size, int startBlock)
    {
        return static_cast<int>(mm_swapper_manager_free(
            this->map,
            size,
            static_cast<unsigned long>(startBlock),
            BLOCK_SIZE
        ));
    }

public:
    MapNode map[SwapperManager::SWAPPER_MAP_ARRAY_SIZE];
};

#endif
