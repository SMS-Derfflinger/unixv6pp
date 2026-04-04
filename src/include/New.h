#ifndef NEW_H
#define NEW_H

#include "KernelAllocator.h"

#ifdef __cplusplus
extern "C" {
#endif

unsigned long mm_new_alloc(MapNode map[], unsigned long size);
void mm_new_free(MapNode map[], unsigned long ptr);

#ifdef __cplusplus
}
#endif

void set_kernel_allocator(KernelAllocator* pAllocator);
void* operator new(unsigned int size);
void operator delete(void* p);
void operator delete(void* p, unsigned int n);

#endif
