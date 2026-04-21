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
            "pop   %eax", // return address
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

    // /// Save the current calling frame and "fork" the execution flow. That is to say: return
    // /// immediately with false. And subsequent callers of `switch()` will see exactly the same
    // /// calling frame and return to the calling site (which is here) with true.
    // ///
    // /// # Safety
    // /// The caller MUST ensure that the stack is pinned and kept the same when we return here by
    // /// calling `switch()`.
    // #[unsafe(naked)]
    // pub unsafe extern "C" fn save(&mut self) -> bool {
    //     naked_asm!(
    //         "mov 4(%esp), %ecx",
    //         "mov %esp,   (%ecx)",
    //         "mov %ebp,  4(%ecx)",
    //         "mov %ebx,  8(%ecx)",
    //         "mov %esi, 12(%ecx)",
    //         "mov %edi, 16(%ecx)",
    //         "xor %eax, %eax",
    //         "pop %edx",
    //         "mov %edx, 20(%ecx)",
    //         "jmp *%edx",
    //         options(att_syntax),
    //     )
    // }

    // /// Load the saved context and return to where we came from (`save()` calls).
    // ///
    // /// Note that this function **NEVER** returns.
    // ///
    // /// # Safety
    // /// The caller MUST ensure that the context is valid (i.e. containing valid stack frame and the
    // /// content of the stack is unchanged).
    // #[unsafe(naked)]
    // pub unsafe extern "C" fn load(&mut self) -> ! {
    //     naked_asm!("", options(att_syntax),)
    // }
}
