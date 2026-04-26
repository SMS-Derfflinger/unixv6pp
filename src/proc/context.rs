use core::arch::naked_asm;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    s: [usize; 12],
    sp: usize,
    ra: usize,
    sstatus: usize,
}

impl TaskContext {
    const SSTATUS_SUM: usize = 1 << 18;

    pub const fn new() -> Self {
        Self {
            s: [0; 12],
            sp: 0,
            ra: 0,
            sstatus: Self::SSTATUS_SUM,
        }
    }

    pub fn set_program_counter(&mut self, pc: usize) {
        self.ra = pc;
    }

    pub fn set_stack_pointer(&mut self, sp: usize) {
        self.sp = sp;
    }

    pub fn is_interrupt_enabled(&self) -> bool {
        (self.sstatus & (1 << 1)) != 0
    }

    pub fn set_interrupt_enabled(&mut self, enabled: bool) {
        if enabled {
            self.sstatus |= 1 << 1;
        } else {
            self.sstatus &= !(1 << 1);
        }
    }

    /// Save current context to `from` and load context from `to`.
    #[unsafe(naked)]
    pub unsafe extern "C" fn switch(from: &mut Self, to: &mut Self) {
        naked_asm!(
            "sd   s0,   0(a0)",
            "sd   s1,   8(a0)",
            "sd   s2,  16(a0)",
            "sd   s3,  24(a0)",
            "sd   s4,  32(a0)",
            "sd   s5,  40(a0)",
            "sd   s6,  48(a0)",
            "sd   s7,  56(a0)",
            "sd   s8,  64(a0)",
            "sd   s9,  72(a0)",
            "sd  s10,  80(a0)",
            "sd  s11,  88(a0)",
            "sd   sp,  96(a0)",
            "sd   ra, 104(a0)",
            "csrr t0, sstatus",
            "sd   t0, 112(a0)",
            "",
            "ld   s0,   0(a1)",
            "ld   s1,   8(a1)",
            "ld   s2,  16(a1)",
            "ld   s3,  24(a1)",
            "ld   s4,  32(a1)",
            "ld   s5,  40(a1)",
            "ld   s6,  48(a1)",
            "ld   s7,  56(a1)",
            "ld   s8,  64(a1)",
            "ld   s9,  72(a1)",
            "ld  s10,  80(a1)",
            "ld  s11,  88(a1)",
            "ld   sp,  96(a1)",
            "ld   ra, 104(a1)",
            "ld   t0, 112(a1)",
            "csrw sstatus, t0",
            "ret",
        );
    }
}
