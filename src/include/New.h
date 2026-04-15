#ifndef NEW_H
#define NEW_H

void* operator new(unsigned int size);
void* operator new(__SIZE_TYPE__, void* ptr);
void operator delete(void* p);
void operator delete(void* p, unsigned int n);

#endif
