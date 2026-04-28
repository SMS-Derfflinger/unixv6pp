#ifndef LIB_SRC_SYSCALL_H
#define LIB_SRC_SYSCALL_H

static inline long __syscall0(long number)
{
	register long a0 __asm__("a0") = 0;
	register long a7 __asm__("a7") = number;
	__asm__ volatile ("ecall" : "+r"(a0) : "r"(a7) : "memory");
	return a0;
}

static inline long __syscall1(long number, long arg0)
{
	register long a0 __asm__("a0") = arg0;
	register long a7 __asm__("a7") = number;
	__asm__ volatile ("ecall" : "+r"(a0) : "r"(a7) : "memory");
	return a0;
}

static inline long __syscall2(long number, long arg0, long arg1)
{
	register long a0 __asm__("a0") = arg0;
	register long a1 __asm__("a1") = arg1;
	register long a7 __asm__("a7") = number;
	__asm__ volatile ("ecall" : "+r"(a0) : "r"(a1), "r"(a7) : "memory");
	return a0;
}

static inline long __syscall3(long number, long arg0, long arg1, long arg2)
{
	register long a0 __asm__("a0") = arg0;
	register long a1 __asm__("a1") = arg1;
	register long a2 __asm__("a2") = arg2;
	register long a7 __asm__("a7") = number;
	__asm__ volatile ("ecall" : "+r"(a0) : "r"(a1), "r"(a2), "r"(a7) : "memory");
	return a0;
}

static inline long __syscall_ret(long result)
{
	return result >= 0 ? result : -1;
}

#endif
