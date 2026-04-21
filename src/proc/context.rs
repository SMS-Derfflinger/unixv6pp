use core::arch::naked_asm;

#[repr(C)]
pub struct TaskContext {
    esp: usize,
    ebp: usize,
    ebx: usize,
    esi: usize,
    edi: usize,
    eip: usize,
}

impl TaskContext {
    /// Save the current calling frame and "fork" the execution flow. That is to say: return
    /// immediately with false. And subsequent callers of `switch()` will see exactly the same
    /// calling frame and return to the calling site (which is here) with true.
    ///
    /// # Safety
    /// The caller MUST ensure that the stack is pinned and kept the same when we return here by
    /// calling `switch()`.
    #[unsafe(naked)]
    unsafe extern "C" fn save(&mut self) -> bool {
        naked_asm!(
            "mov 4(%esp), %ecx",
            "mov %esp,   (%ecx)",
            "mov %ebp,  4(%ecx)",
            "mov %ebx,  8(%ecx)",
            "mov %esi, 12(%ecx)",
            "mov %edi, 16(%ecx)",
            "xor %eax, %eax",
            "pop %edx",
            "mov %edx, 20(%ecx)",
            "jmp *%edx",
            options(att_syntax),
        )
    }

    /// Load the saved context and return to where we came from (`save()` calls).
    ///
    /// Note that this function **NEVER** returns.
    ///
    /// # Safety
    /// The caller MUST ensure that the context is valid (i.e. containing valid stack frame and the
    /// content of the stack is unchanged).
    #[unsafe(naked)]
    unsafe extern "C" fn load(&mut self) -> ! {
        naked_asm!(
            "",
            options(att_syntax),
        )
    }
}
