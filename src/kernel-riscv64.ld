OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    RAM : ORIGIN = 0x80200000, LENGTH = 8M
}

SECTIONS
{
    . = ORIGIN(RAM);

    .text : ALIGN(16)
    {
        KEEP(*(.text._start))
        KEEP(*(.text._start.*))
        *(.text .text.*)
    } > RAM

    .rodata : ALIGN(16)
    {
        *(.rodata .rodata.*)
    } > RAM

    .data : ALIGN(16)
    {
        *(.data .data.*)
    } > RAM

    .bss : ALIGN(16)
    {
        __bss_start = .;
        *(.bss.stack)
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > RAM

    /DISCARD/ :
    {
        *(.eh_frame*)
        *(.comment*)
        *(.note*)
    }
}
