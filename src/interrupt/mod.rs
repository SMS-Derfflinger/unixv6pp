mod exception;
mod interrupt;

use core::arch::asm;

use crate::dev::io_port::IOPort;

pub const KERNEL_MODE: u32 = 0x0;
pub const USER_MODE: u32 = 0x3;

const PIC_MASTER_IO_PORT_1: u16 = 0x20;
const PIC_EOI: u8 = 0x20;

#[repr(C)]
pub struct PtRegs {
    pub pad1: u32,
    pub pad2: u32,
    pub xds: u32,
    pub xes: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub eax: u32,
}

#[repr(C)]
pub struct PtContext {
    pub eip: u32,
    pub xcs: u32,
    pub eflags: u32,
    pub esp: u32,
    pub xss: u32,
}

#[repr(C)]
pub struct PteContext {
    pub error_code: u32,
    pub eip: u32,
    pub xcs: u32,
    pub eflags: u32,
    pub esp: u32,
    pub xss: u32,
}

impl PtContext {
    pub fn from_user_mode(&self) -> bool {
        self.xcs & USER_MODE == USER_MODE
    }
}

impl PteContext {
    pub fn as_context(&mut self) -> *mut PtContext {
        core::ptr::addr_of_mut!(self.eip).cast::<PtContext>()
    }

    pub fn from_user_mode(&self) -> bool {
        self.xcs & USER_MODE == USER_MODE
    }
}

pub fn send_master_eoi() {
    unsafe {
        IOPort::out_byte(PIC_MASTER_IO_PORT_1, PIC_EOI);
    }
}

pub unsafe fn switch_to_kernel_segments() {
    unsafe {
        asm!(
            "movw $0x10, %dx",
            "movw %dx, %ds",
            "movw %dx, %es",
            options(att_syntax),
        );
    }
}

#[macro_export]
macro_rules! interrupt_entry_no_segment_switch {
    ($entry:ident, $handler:ident) => {
        core::arch::global_asm!(
            concat!(
                ".text\n",
                ".global ", stringify!($entry), "\n",
                ".type ", stringify!($entry), ", @function\n",
                stringify!($entry), ":\n",
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
                "    call ", stringify!($handler), "\n",
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
                ".size ", stringify!($entry), ", . - ", stringify!($entry), "\n",
            ),
            options(att_syntax),
        );
    };
}

#[macro_export]
macro_rules! interrupt_entry {
    ($entry:ident, $handler:ident) => {
        core::arch::global_asm!(
            concat!(
                ".text\n",
                ".global ", stringify!($entry), "\n",
                ".type ", stringify!($entry), ", @function\n",
                stringify!($entry), ":\n",
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
                "    call ", stringify!($handler), "\n",
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
                ".size ", stringify!($entry), ", . - ", stringify!($entry), "\n",
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
                ".global ", stringify!($entry), "\n",
                ".type ", stringify!($entry), ", @function\n",
                stringify!($entry), ":\n",
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
                "    lea 0x8(%ebp), %edx\n",
                "    pushl %edx\n",
                "    lea 0x4(%esp), %edx\n",
                "    pushl %edx\n",
                "    movw $0x10, %dx\n",
                "    movw %dx, %ds\n",
                "    movw %dx, %es\n",
                "    call ", stringify!($handler), "\n",
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
                ".size ", stringify!($entry), ", . - ", stringify!($entry), "\n",
            ),
            options(att_syntax),
        );
    };
}
