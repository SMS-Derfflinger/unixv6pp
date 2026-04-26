OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    RAM : ORIGIN = 0x80200000, LENGTH = 8M
}

SECTIONS
{
    . = ORIGIN(RAM);
    .bootstrap ORIGIN(RAM) :
    {
        KEEP(*(.bootstrap.entry));

        . = ALIGN(16);
        KEEP(*(.bootstrap.stack));
    } > RAM

    .text : ALIGN(16)
    {
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
