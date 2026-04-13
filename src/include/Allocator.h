#ifndef ALLOCATOR_H
#define ALLOCATOR_H

#ifdef __cplusplus
extern "C" {
#endif

#ifdef __cplusplus
}
#endif

class Allocator
{
public:
    static Allocator& GetInstance()
    {
        static Allocator instance;
        return instance;
    }
};

#endif
