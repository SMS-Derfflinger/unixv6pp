mod exception;
mod interrupt;
mod system_call;
mod time;

pub use interrupt::send_master_eoi;
pub use time::{get_time, set_time};

pub const KERNEL_MODE: u32 = 0x0;
pub const USER_MODE: u32 = 0x3;

const PIC_MASTER_IO_PORT_1: u16 = 0x20;
const PIC_EOI: u8 = 0x20;

#[repr(C)]
pub struct Registers {
    _we_dont_care: [u32; 4],
    pub ebx: usize,
    pub ecx: usize,
    pub edx: usize,
    pub esi: usize,
    pub edi: usize,
    _ebp: usize,
    pub eax: usize,
    pub ebp: usize,
}

#[macro_export]
macro_rules! interrupt_entry {
    ($entry:ident, $handler:ident) => {
        core::arch::global_asm!(
            concat!(
                ".text\n",
                ".global ",
                stringify!($entry),
                "\n",
                ".type ",
                stringify!($entry),
                ", @function\n",
                stringify!($entry),
                ":\n",
                "    pushl %ebp\n",
                "    movl %esp, %ebp\n",
                "    cld\n",
                "    pushl %eax\n",
                "    pushl %ebp\n",
                "    pushl %edi\n",
                "    pushl %esi\n",
                "    pushl %edx\n",
                "    pushl %ecx\n",
                "    pushl %ebx\n",
                "    pushl %es\n",
                "    pushl %ds\n",
                "    pushl %fs\n",
                "    pushl %gs\n",
                "    lea 0x4(%ebp), %edx\n",
                "    pushl %edx\n",
                "    lea 0x4(%esp), %edx\n",
                "    pushl %edx\n",
                "    movw $0x10, %dx\n",
                "    movw %dx, %ds\n",
                "    movw %dx, %es\n",
                "    call ",
                stringify!($handler),
                "\n",
                "    addl $0x8, %esp\n",
                "    popl %gs\n",
                "    popl %fs\n",
                "    popl %ds\n",
                "    popl %es\n",
                "    popl %ebx\n",
                "    popl %ecx\n",
                "    popl %edx\n",
                "    popl %esi\n",
                "    popl %edi\n",
                "    popl %ebp\n",
                "    popl %eax\n",
                "    leave\n",
                "    iret\n",
                ".size ",
                stringify!($entry),
                ", . - ",
                stringify!($entry),
                "\n",
            ),
            options(att_syntax),
        );
    };
}

#[macro_export]
macro_rules! interrupt_entry_with_error_code {
    ($entry:ident, $handler:ident) => {
        core::arch::global_asm!(
            concat!(
                ".text\n",
                ".global ",
                stringify!($entry),
                "\n",
                ".type ",
                stringify!($entry),
                ", @function\n",
                stringify!($entry),
                ":\n",
                "    pushl %ebp\n",
                "    movl %esp, %ebp\n",
                "    cld\n",
                "    pushl %eax\n",
                "    pushl %ebp\n",
                "    pushl %edi\n",
                "    pushl %esi\n",
                "    pushl %edx\n",
                "    pushl %ecx\n",
                "    pushl %ebx\n",
                "    pushl %es\n",
                "    pushl %ds\n",
                "    pushl %fs\n",
                "    pushl %gs\n",
                "    lea 0x4(%ebp), %edx\n",
                "    pushl %edx\n",
                "    lea 0x4(%esp), %edx\n",
                "    pushl %edx\n",
                "    movw $0x10, %dx\n",
                "    movw %dx, %ds\n",
                "    movw %dx, %es\n",
                "    call ",
                stringify!($handler),
                "\n",
                "    addl $0x8, %esp\n",
                "    popl %gs\n",
                "    popl %fs\n",
                "    popl %ds\n",
                "    popl %es\n",
                "    popl %ebx\n",
                "    popl %ecx\n",
                "    popl %edx\n",
                "    popl %esi\n",
                "    popl %edi\n",
                "    popl %ebp\n",
                "    popl %eax\n",
                "    leave\n",
                "    addl $4, %esp\n",
                "    iret\n",
                ".size ",
                stringify!($entry),
                ", . - ",
                stringify!($entry),
                "\n",
            ),
            options(att_syntax),
        );
    };
}
