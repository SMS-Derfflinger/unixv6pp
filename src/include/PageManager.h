#ifndef PAGE_MANAGER_H
#define PAGE_MANAGER_H

#ifdef __cplusplus
extern "C" {
#endif

unsigned long alloc_page(unsigned long size, bool is_user);
void free_page(unsigned long addr, unsigned long size, bool is_user);

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
