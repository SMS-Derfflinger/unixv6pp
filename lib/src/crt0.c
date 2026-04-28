__attribute__((naked, noreturn)) void _start(void)
{
	__asm__ volatile(
		".option push\n"
		".option norelax\n"
		"la gp, __global_pointer$\n"
		".option pop\n"
		"call main1\n"
		"mv a0, a0\n"
		"li a7, 1\n"
		"ecall\n"
		"1: j 1b\n"
	);
}
