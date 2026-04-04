#include "New.h"

KernelAllocator* g_pAllocator;

void set_kernel_allocator(KernelAllocator* pAllocator)
{
    g_pAllocator = pAllocator;
}

void* operator new(unsigned int size)
{
    if (g_pAllocator == nullptr) {
        return nullptr;
    }

    unsigned long address = mm_new_alloc(g_pAllocator->map, size);
    if (address == 0) {
        return nullptr;
    }

    return reinterpret_cast<void*>(address);
}

void operator delete(void* p)
{
    if (g_pAllocator == nullptr) {
        return;
    }

    mm_new_free(g_pAllocator->map, reinterpret_cast<unsigned long>(p));
}

void operator delete(void* p, unsigned int n)
{
    (void)n;
    if (g_pAllocator == nullptr) {
        return;
    }

    mm_new_free(g_pAllocator->map, reinterpret_cast<unsigned long>(p));
}
