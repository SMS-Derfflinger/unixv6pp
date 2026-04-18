#ifndef TSS_H
#define TSS_H

extern "C" void _task_state_segment_init();

struct TaskStateSegment
{
public:
    void Initialize() {
        _task_state_segment_init();
    }
};

#endif
