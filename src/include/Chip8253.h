#ifndef CHIP8253_H
#define CHIP8253_H

extern "C" void _chip8253_init(int ticks);

/*
 * 定义对8253可编程定时芯片(PIT)的操作。
 *
 * 8253芯片用于产生固定间隔的时钟中断。
 */
class Chip8253
{
public:
	static void Init(int ticks = 60) {
        _chip8253_init(ticks);
    }
};

#endif
