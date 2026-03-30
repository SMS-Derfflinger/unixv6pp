#include "MapNode.h"

#ifdef __cplusplus
extern "C" {
#endif

unsigned long mm_allocator_alloc(MapNode map[], unsigned long size);
unsigned long mm_allocator_free(MapNode map[], unsigned long size, unsigned long addr_idx);

#ifdef __cplusplus
}
#endif
