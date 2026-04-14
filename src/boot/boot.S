#include <asm_defines.h>

.code16

start_16bit:

mov $0x7c00, %esp

#ifdef USE_VESA

xor %ax, %ax
mov %ax, %es

mov $0x7e00, %di
mov $0x4f01, %ax
mov $0x0143, %cx
int $0x10

mov $0x4143, %bx
mov $0x4f02, %ax
int $0x10

#endif

lgdt early_gdt_desc

cli

// enable A20
in $0x92, %al

// set PE
mov %cr0, %eax
or $0x1, %eax
mov %eax, %cr0

// set PSE
mov %cr4, %eax
or $0x10, %eax
mov %eax, %cr4

ljmp $0x08, $start_32bit

.code32

start_32bit:

mov $0x10, %ax
mov %ax, %ds
mov %ax, %es
mov %ax, %ss
mov $ASM_KERNEL_SECTORS, %ecx

mov $0x01, %eax
mov $0x100000, %ebx

2:

push %eax
inc %eax
push %ebx
add $0x200, %ebx

call read_disk
add $0x8, %esp

loop 2b

movw $0x20, %ax
mov %ax, %ds
mov %ax, %es
mov %ax, %ss
or $0xc0000000, %esp

ljmp $0x18, $0xc0100000

read_disk:
push %ebp
mov %esp, %ebp

push %edx
push %ecx
push %edi
push %eax

mov $0x1, %al
mov $0x01f2, %dx
out %al, %dx

mov 12(%ebp), %eax
inc %dx
out %al, %dx

shr $0x8, %eax
inc %dx
out %al, %dx

shr $0x8, %eax
inc %dx
out %al, %dx

shr $0x8, %eax
and $0x0f, %al

or $0xe0, %al
inc %dx
out %al, %dx

mov $0x20, %al
inc %dx
out %al, %dx

2:
in %dx, %al
test $0x80, %al
jnz 2b

test $0x08, %al
jz 2f

mov $0x80, %ecx
mov $0x01f0, %dx
mov 8(%ebp), %edi
rep insl

xor %ax, %ax
jmp 3f

2:
mov $0x01f1, %dx
in %dx, %al
xor %ah, %ah

3:
pop %eax
pop %edi
pop %ecx
pop %edx

mov %ebp, %esp
pop %ebp
ret

.align 4
early_gdt:
.quad 0x0000000000000000
.quad 0x00cf9a000000ffff
.quad 0x00cf92000000ffff
.quad 0x40cf9a000000ffff
.quad 0x40cf92000000ffff

.Learly_gdt_end:

.align 2
early_gdt_desc:
.word .Learly_gdt_end - early_gdt // limit
.long early_gdt     // address

// some marker for no reason
.word 0xabfb

.org 510, 0

.byte 0x55
.byte 0xaa
