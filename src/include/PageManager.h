#ifndef PAGE_MANAGER_H
#define PAGE_MANAGER_H

#include "MapNode.h"
#include "Allocator.h"

#ifdef __cplusplus
extern "C" {
#endif

unsigned long mm_page_manager_initialize(MapNode map[], unsigned long map_len);
unsigned long mm_page_manager_init_pool(
    MapNode map[],
    unsigned long map_len,
    unsigned long page_size,
    unsigned long pool_start_addr,
    unsigned long pool_size
);
unsigned long mm_page_manager_alloc(MapNode map[], unsigned long size, unsigned long page_size);
unsigned long mm_page_manager_free(
    MapNode map[],
    unsigned long size,
    unsigned long start_address,
    unsigned long page_size
);

#ifdef __cplusplus
}
#endif

class PageManager
{
public:
    inline static unsigned int PHY_MEM_SIZE = 0;

    static const unsigned int PAGE_SIZE = 0x1000;
    static const unsigned int MEMORY_MAP_ARRAY_SIZE = 0x200;
    static const unsigned int KERNEL_MEM_START_ADDR = 0x100000;
    static const unsigned int KERNEL_SIZE = 0x80000;

public:
    PageManager(Allocator* allocator)
    {
        (void)allocator;
    }

    virtual ~PageManager() = default;

    int Initialize()
    {
        return static_cast<int>(mm_page_manager_initialize(this->map, MEMORY_MAP_ARRAY_SIZE));
    }

    unsigned long AllocMemory(unsigned long size)
    {
        return mm_page_manager_alloc(this->map, size, PAGE_SIZE);
    }

    unsigned long FreeMemory(unsigned long size, unsigned long memoryStartAddress)
    {
        return mm_page_manager_free(this->map, size, memoryStartAddress, PAGE_SIZE);
    }

public:
    MapNode map[PageManager::MEMORY_MAP_ARRAY_SIZE];
};

class KernelPageManager : public PageManager
{
public:
    static const unsigned int KERNEL_PAGE_POOL_START_ADDR = 0x200000 + 0x2000 + 0x2000;
    static const unsigned int KERNEL_PAGE_POOL_SIZE = 0x200000 - 0x4000;

public:
    KernelPageManager(Allocator* allocator)
        : PageManager(allocator)
    {
    }

    int Initialize()
    {
        return static_cast<int>(mm_page_manager_init_pool(
            this->map,
            MEMORY_MAP_ARRAY_SIZE,
            PAGE_SIZE,
            KERNEL_PAGE_POOL_START_ADDR,
            KERNEL_PAGE_POOL_SIZE
        ));
    }
};

class UserPageManager : public PageManager
{
public:
    static const unsigned int USER_PAGE_POOL_START_ADDR = 0x400000;
    inline static unsigned int USER_PAGE_POOL_SIZE = 0;

public:
    UserPageManager(Allocator* allocator)
        : PageManager(allocator)
    {
    }

    int Initialize()
    {
        return static_cast<int>(mm_page_manager_init_pool(
            this->map,
            MEMORY_MAP_ARRAY_SIZE,
            PAGE_SIZE,
            USER_PAGE_POOL_START_ADDR,
            USER_PAGE_POOL_SIZE
        ));
    }
};

#endif
