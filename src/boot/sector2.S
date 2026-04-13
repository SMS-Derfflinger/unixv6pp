.section .sector2
.code32

.extern kernelBridge

.globl greatstart
greatstart:
	mov $0xc0001000, %edi
	mov %edi, %ebx
	mov $0x400, %ecx
	call fill

	mov $0x83, %eax
	mov %eax, (%ebx)
	mov %eax, 0xc00(%ebx)

	lidt empty_idt_desc
	mov $0x1000, %edx
	mov %edx, %cr3

	lgdt gdt_desc

	mov %cr0, %eax
	or $0x80000000, %eax
	mov %eax, %cr0

	ljmp $0x8, $2f
2:
	mov $0xc0007c00, %esp
	jmp kernelBridge
	ud2

fill:
	xor %eax, %eax
2:
	mov %eax, (%edi)
	add $4, %edi
	loop 2b
	ret

.align 4
empty_idt_desc:
	.word 0
	.long 0

.align 4
gdt_desc:
	.word .Lgdt_end - gdt
	.long gdt
	.long 0

.align 8
gdt:
	.quad 0
	.quad 0x00cf9a000000ffff
	.quad 0x00cf92000000ffff
.Lgdt_end:
