use core::arch::naked_asm;

use crate::{
    machine::{asm, Registers, TrapContext},
    println_fatal, println_info,
};

static mut BOOT_TRAP_CONTEXT: TrapContext = TrapContext::new();

pub fn init_trap() {
    let trap_context = &raw mut BOOT_TRAP_CONTEXT as *mut TrapContext as usize;
    asm::write_sscratch(trap_context);
    asm::write_stvec(_trap_entry as *const () as usize);
}

#[unsafe(naked)]
unsafe extern "C" fn _trap_entry() -> ! {
    naked_asm!(
        "csrrw t0, sscratch, t0",
        "sd    tp,   {tp}(t0)",
        "sd    ra,   {ra}(t0)",
        "sd    sp,   {sp}(t0)",
        "sd    gp,   {gp}(t0)",
        "sd    a0,   {a0}(t0)",
        "sd    a1,   {a1}(t0)",
        "sd    a2,   {a2}(t0)",
        "sd    a3,   {a3}(t0)",
        "sd    a4,   {a4}(t0)",
        "sd    t1,   {t1}(t0)",
        "sd    a5,   {a5}(t0)",
        "sd    a6,   {a6}(t0)",
        "sd    a7,   {a7}(t0)",
        "sd    t3,   {t3}(t0)",
        "sd    t4,   {t4}(t0)",
        "sd    t5,   {t5}(t0)",
        "sd    t2,   {t2}(t0)",
        "sd    t6,   {t6}(t0)",
        "sd    s0,   {s0}(t0)",
        "sd    s1,   {s1}(t0)",
        "sd    s2,   {s2}(t0)",
        "sd    s3,   {s3}(t0)",
        "sd    s4,   {s4}(t0)",
        "sd    s5,   {s5}(t0)",
        "sd    s6,   {s6}(t0)",
        "sd    s7,   {s7}(t0)",
        "sd    s8,   {s8}(t0)",
        "sd    s9,   {s9}(t0)",
        "sd   s10,  {s10}(t0)",
        "sd   s11,  {s11}(t0)",
        "mv    a0, t0",
        "csrrw t0, sscratch, t0",
        "sd    t0,   {t0}(a0)",
        "csrr  t0, sepc",
        "csrr  t1, scause",
        "csrr  t2, sstatus",
        "csrr  t3, stval",
        "sd    t0, {sepc}(a0)",
        "sd    t1, {scause}(a0)",
        "sd    t2, {sstatus}(a0)",
        "sd    t3, {stval}(a0)",
        "call {trap_handler}",
        "csrr  t0, sscratch",
        "ld    t1, {sepc}(t0)",
        "ld    t2, {sstatus}(t0)",
        "ld    tp,   {tp}(t0)",
        "ld    ra,   {ra}(t0)",
        "ld    sp,   {sp}(t0)",
        "ld    gp,   {gp}(t0)",
        "ld    a0,   {a0}(t0)",
        "ld    a1,   {a1}(t0)",
        "ld    a2,   {a2}(t0)",
        "ld    a3,   {a3}(t0)",
        "ld    a4,   {a4}(t0)",
        "csrw  sepc, t1",
        "csrw  sstatus, t2",
        "ld    t1,   {t1}(t0)",
        "ld    a5,   {a5}(t0)",
        "ld    a6,   {a6}(t0)",
        "ld    a7,   {a7}(t0)",
        "ld    t3,   {t3}(t0)",
        "ld    t4,   {t4}(t0)",
        "ld    t5,   {t5}(t0)",
        "ld    t2,   {t2}(t0)",
        "ld    t6,   {t6}(t0)",
        "ld    s0,   {s0}(t0)",
        "ld    s1,   {s1}(t0)",
        "ld    s2,   {s2}(t0)",
        "ld    s3,   {s3}(t0)",
        "ld    s4,   {s4}(t0)",
        "ld    s5,   {s5}(t0)",
        "ld    s6,   {s6}(t0)",
        "ld    s7,   {s7}(t0)",
        "ld    s8,   {s8}(t0)",
        "ld    s9,   {s9}(t0)",
        "ld   s10,  {s10}(t0)",
        "ld   s11,  {s11}(t0)",
        "ld    t0,   {t0}(t0)",
        "sret",
        tp = const Registers::OFFSET_TP,
        ra = const Registers::OFFSET_RA,
        sp = const Registers::OFFSET_SP,
        gp = const Registers::OFFSET_GP,
        a0 = const Registers::OFFSET_A0,
        a1 = const Registers::OFFSET_A1,
        a2 = const Registers::OFFSET_A2,
        a3 = const Registers::OFFSET_A3,
        a4 = const Registers::OFFSET_A4,
        t1 = const Registers::OFFSET_T1,
        a5 = const Registers::OFFSET_A5,
        a6 = const Registers::OFFSET_A6,
        a7 = const Registers::OFFSET_A7,
        t3 = const Registers::OFFSET_T3,
        t4 = const Registers::OFFSET_T4,
        t5 = const Registers::OFFSET_T5,
        t2 = const Registers::OFFSET_T2,
        t6 = const Registers::OFFSET_T6,
        s0 = const Registers::OFFSET_S0,
        s1 = const Registers::OFFSET_S1,
        s2 = const Registers::OFFSET_S2,
        s3 = const Registers::OFFSET_S3,
        s4 = const Registers::OFFSET_S4,
        s5 = const Registers::OFFSET_S5,
        s6 = const Registers::OFFSET_S6,
        s7 = const Registers::OFFSET_S7,
        s8 = const Registers::OFFSET_S8,
        s9 = const Registers::OFFSET_S9,
        s10 = const Registers::OFFSET_S10,
        s11 = const Registers::OFFSET_S11,
        t0 = const Registers::OFFSET_T0,
        sstatus = const TrapContext::OFFSET_SSTATUS,
        sepc = const TrapContext::OFFSET_SEPC,
        scause = const TrapContext::OFFSET_SCAUSE,
        stval = const TrapContext::OFFSET_STVAL,
        trap_handler = sym riscv64_trap_handler,
    );
}

extern "C" fn riscv64_trap_handler(context: &mut TrapContext) {
    if context.is_interrupt() {
        println_info!(
            "trap: interrupt cause={} sepc={:#x} stval={:#x}",
            context.cause_code(),
            context.sepc,
            context.stval
        );
        return;
    }

    if context.cause_code() == 3 {
        println_info!(
            "trap: breakpoint sepc={:#x} stval={:#x}",
            context.sepc,
            context.stval
        );
        context.advance_sepc(2);
        return;
    }

    println_fatal!(
        "trap: exception cause={} sepc={:#x} stval={:#x} user={}",
        context.cause_code(),
        context.sepc,
        context.stval,
        context.is_user()
    );

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
