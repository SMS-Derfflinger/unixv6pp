#include <sys.h>
#include <malloc.h>
#include <stdio.h>

#define PAGE_SIZE 4096

char *malloc_begin = NULL;
char *malloc_end = NULL;

typedef struct flist {
   unsigned int size;
   struct flist *nlink;
} flist;

struct flist *malloc_head = NULL;

void* malloc(unsigned long size)
{
    if (malloc_begin == NULL)
    {
        malloc_begin = (char*) sbrk(0);
        if (malloc_begin == (void*) -1 || sbrk(PAGE_SIZE) == (void*) -1)
        {
            return NULL;
        }
        malloc_end = malloc_begin + PAGE_SIZE;
        malloc_head = (void*) malloc_begin;
        malloc_head->size = sizeof(struct flist);
        malloc_head->nlink = NULL;
    }
    if (size == 0)
    {
        return NULL;
    }
    size += sizeof(struct flist);
    size = ((size + 7) >> 3) << 3;
    struct flist* iter = malloc_head;
    // find a place to insert
    while(iter->nlink)
    {
        if ((unsigned long) ((char *) iter->nlink - ((char *) iter + iter->size)) >= size)
        {
            struct flist *temp = (void*) ((char *)iter + (iter->size));
            temp->nlink = iter->nlink;
            iter->nlink = temp;
            temp->size = size;
            return (char *)temp + sizeof(struct flist);
        }
        iter = iter->nlink;
    }
    // not found
    long expand = (long) size - (malloc_end - (char *) iter - iter->size);
    if (expand < 0)
    {
        expand = 0;
    }
    else
    {
        expand = ((expand + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
    }
    if (expand > 0)
    {
        if (sbrk(expand) == (void*) -1)
        {
            return NULL;
        }
        malloc_end += expand;
    }
    iter->nlink = (void*) ((char *)iter + (iter->size));
    iter = iter->nlink;
    iter->size = size;
    iter->nlink = NULL;
    printf("%p\n", iter);
    return (char*)iter + sizeof(struct flist);
}

int free(void* addr)
{
    char * real_addr = (char *) addr - sizeof(struct flist);
    struct flist* iter = malloc_head;
    struct flist* last = malloc_head;
    if (addr == 0)
    {
        return -1;
    }
    // find a place to insert
    while(iter)
    {
        if ((void*) iter == (void*) real_addr)
        {
            last->nlink = iter->nlink;
            if (last->nlink == NULL)
            {
                char *pos = (char *)last + last->size;
                if (malloc_end - pos > PAGE_SIZE * 2)
                {
                    long shrink = (malloc_end - pos) / PAGE_SIZE * PAGE_SIZE;
                    if (sbrk(-shrink) != (void*) -1)
                    {
                        malloc_end -= shrink;
                    }
                }
            }
            return 0;
        }
        last = iter;
        iter = iter->nlink;
    }
    return -1;
}

