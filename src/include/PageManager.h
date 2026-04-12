#ifndef PAGE_MANAGER_H
#define PAGE_MANAGER_H

#include "Allocator.h"
#include "MapNode.h"
#include "Video.h"

#ifdef __cplusplus
extern "C" {
#endif

unsigned long alloc_page(unsigned long size, bool is_user);
void free_page(unsigned long addr, unsigned long size, bool is_user);

unsigned long mm_page_manager_initialize(MapNode map[], unsigned long map_len);
unsigned long mm_page_manager_init_pool(MapNode map[], unsigned long map_len,
                                        unsigned long page_size,
                                        unsigned long pool_start_addr,
                                        unsigned long pool_size);
unsigned long mm_page_manager_alloc(MapNode map[], unsigned long size,
                                    unsigned long page_size);
unsigned long mm_page_manager_free(MapNode map[], unsigned long size,
                                   unsigned long start_address,
                                   unsigned long page_size);

#ifdef __cplusplus
}
#endif

class KernelPageManager {
public:
  KernelPageManager() {}

  unsigned long AllocMemory(unsigned long size) {
          return alloc_page(size, false);
  }

  void FreeMemory(unsigned long size, unsigned long memoryStartAddress) {
          free_page(memoryStartAddress, size, false);
  }
};

class UserPageManager {
public:
  static const unsigned int USER_PAGE_POOL_START_ADDR = 0x400000;
  inline static unsigned int USER_PAGE_POOL_SIZE = 0;

public:
  UserPageManager() {}

  unsigned long AllocMemory(unsigned long size) {
          return alloc_page(size, true);
  }

  void FreeMemory(unsigned long size, unsigned long memoryStartAddress) {
          free_page(memoryStartAddress, size, true);
  }
};

#endif
