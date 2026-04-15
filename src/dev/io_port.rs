use core::arch::asm;

pub struct IOPort;

// TODO: only for x86
impl IOPort {
    #[inline]
    pub unsafe fn in_byte(port: u16) -> u8 {
        let data: u8;
        asm!(
            "inb %dx, %al",
            out("al") data,
            in("dx") port,
            options(nomem, nostack, att_syntax)
        );
        data
    }

    #[inline]
    pub unsafe fn in_word(port: u16) -> u16 {
        let data: u16;
        asm!(
            "inw %dx, %ax",
            out("ax") data,
            in("dx") port,
            options(nomem, nostack, att_syntax)
        );
        data
    }

    #[inline]
    pub unsafe fn in_dword(port: u16) -> u32 {
        let data: u32;
        asm!(
            "inl %dx, %eax",
            out("eax") data,
            in("dx") port,
            options(nomem, nostack, att_syntax)
        );
        data
    }

    #[inline]
    pub unsafe fn out_byte(port: u16, data: u8) {
        asm!(
            "outb %al, %dx",
            in("dx") port,
            in("al") data,
            options(nomem, nostack, att_syntax)
        );
    }

    #[inline]
    pub unsafe fn out_word(port: u16, data: u16) {
        asm!(
            "outw %ax, %dx",
            in("dx") port,
            in("ax") data,
            options(nomem, nostack, att_syntax)
        );
    }

    #[inline]
    pub unsafe fn out_dword(port: u16, data: u32) {
        asm!(
            "outl %eax, %dx",
            in("dx") port,
            in("eax") data,
            options(nomem, nostack, att_syntax)
        );
    }
}
