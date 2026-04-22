use core::arch::naked_asm;

#[repr(C)]
pub struct TaskContext {
    pub eip: usize,
    pub esp: usize,
    pub ebp: usize,
    pub ebx: usize,
    pub esi: usize,
    pub edi: usize,
}

impl TaskContext {
    /// Save current context to `from` and load context from `to`.
    #[unsafe(naked)]
    pub unsafe extern "C" fn switch(from: &mut Self, to: &mut Self) {
        naked_asm!(
            "pop   %eax",           // return address
            "mov   (%esp),   %ecx", // from
            "mov  4(%esp),   %edx", // to
            "",
            "mov    %eax,   (%ecx)",
            "mov   (%edx),   %eax",
            "mov    %esp,  4(%ecx)",
            "mov  4(%edx),   %esp",
            "mov    %ebp,  8(%ecx)",
            "mov  8(%edx),   %ebp",
            "mov    %ebx, 12(%ecx)",
            "mov 12(%edx),   %ebx",
            "mov    %esi, 16(%ecx)",
            "mov 16(%edx),   %esi",
            "mov    %edi, 20(%ecx)",
            "mov 20(%edx),   %edi",
            "jmp   *%eax",
            options(att_syntax),
        )
    }

    pub const fn new() -> Self {
        Self {
            esp: 0,
            ebp: 0,
            ebx: 0,
            esi: 0,
            edi: 0,
            eip: 0,
        }
    }
}
