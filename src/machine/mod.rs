pub mod asm;

use core::mem::offset_of;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Registers {
    pub tp: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub t1: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t2: usize,
    pub t6: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t0: usize,
}

impl Registers {
    pub const OFFSET_TP: usize = offset_of!(Registers, tp);
    pub const OFFSET_RA: usize = offset_of!(Registers, ra);
    pub const OFFSET_SP: usize = offset_of!(Registers, sp);
    pub const OFFSET_GP: usize = offset_of!(Registers, gp);
    pub const OFFSET_A0: usize = offset_of!(Registers, a0);
    pub const OFFSET_A1: usize = offset_of!(Registers, a1);
    pub const OFFSET_A2: usize = offset_of!(Registers, a2);
    pub const OFFSET_A3: usize = offset_of!(Registers, a3);
    pub const OFFSET_A4: usize = offset_of!(Registers, a4);
    pub const OFFSET_T1: usize = offset_of!(Registers, t1);
    pub const OFFSET_A5: usize = offset_of!(Registers, a5);
    pub const OFFSET_A6: usize = offset_of!(Registers, a6);
    pub const OFFSET_A7: usize = offset_of!(Registers, a7);
    pub const OFFSET_T3: usize = offset_of!(Registers, t3);
    pub const OFFSET_T4: usize = offset_of!(Registers, t4);
    pub const OFFSET_T5: usize = offset_of!(Registers, t5);
    pub const OFFSET_T2: usize = offset_of!(Registers, t2);
    pub const OFFSET_T6: usize = offset_of!(Registers, t6);
    pub const OFFSET_S0: usize = offset_of!(Registers, s0);
    pub const OFFSET_S1: usize = offset_of!(Registers, s1);
    pub const OFFSET_S2: usize = offset_of!(Registers, s2);
    pub const OFFSET_S3: usize = offset_of!(Registers, s3);
    pub const OFFSET_S4: usize = offset_of!(Registers, s4);
    pub const OFFSET_S5: usize = offset_of!(Registers, s5);
    pub const OFFSET_S6: usize = offset_of!(Registers, s6);
    pub const OFFSET_S7: usize = offset_of!(Registers, s7);
    pub const OFFSET_S8: usize = offset_of!(Registers, s8);
    pub const OFFSET_S9: usize = offset_of!(Registers, s9);
    pub const OFFSET_S10: usize = offset_of!(Registers, s10);
    pub const OFFSET_S11: usize = offset_of!(Registers, s11);
    pub const OFFSET_T0: usize = offset_of!(Registers, t0);
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default)]
pub struct TrapContext {
    pub regs: Registers,
    pub sstatus: usize,
    pub sepc: usize,
    pub scause: usize,
    pub stval: usize,
}

impl TrapContext {
    pub const SSTATUS_SIE: usize = 1 << 1;
    pub const SSTATUS_SPP: usize = 1 << 8;

    pub const OFFSET_SSTATUS: usize = offset_of!(TrapContext, sstatus);
    pub const OFFSET_SEPC: usize = offset_of!(TrapContext, sepc);
    pub const OFFSET_SCAUSE: usize = offset_of!(TrapContext, scause);
    pub const OFFSET_STVAL: usize = offset_of!(TrapContext, stval);

    pub const fn new() -> Self {
        Self {
            regs: Registers {
                tp: 0,
                ra: 0,
                sp: 0,
                gp: 0,
                a0: 0,
                a1: 0,
                a2: 0,
                a3: 0,
                a4: 0,
                t1: 0,
                a5: 0,
                a6: 0,
                a7: 0,
                t3: 0,
                t4: 0,
                t5: 0,
                t2: 0,
                t6: 0,
                s0: 0,
                s1: 0,
                s2: 0,
                s3: 0,
                s4: 0,
                s5: 0,
                s6: 0,
                s7: 0,
                s8: 0,
                s9: 0,
                s10: 0,
                s11: 0,
                t0: 0,
            },
            sstatus: 0,
            sepc: 0,
            scause: 0,
            stval: 0,
        }
    }

    pub fn is_interrupt(&self) -> bool {
        (self.scause >> (usize::BITS - 1)) != 0
    }

    pub fn cause_code(&self) -> usize {
        self.scause & !(1usize << (usize::BITS - 1))
    }

    pub fn is_user(&self) -> bool {
        (self.sstatus & Self::SSTATUS_SPP) == 0
    }

    pub fn advance_sepc(&mut self, bytes: usize) {
        self.sepc = self.sepc.wrapping_add(bytes);
    }
}
