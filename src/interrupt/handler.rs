use core::arch::naked_asm;

use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    ExceptionNumber, InterruptNumber,
};

use crate::{
    constants::{platform::UART0_IRQ, PosixError},
    interrupt::{
        context::{Registers, TrapContext},
        plic, schedule_on_user_return, system_call, time,
    },
    println_fatal, println_info, serial, tty,
};

#[unsafe(naked)]
pub(crate) unsafe extern "C" fn _trap_entry() -> ! {
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
        "andi  t1, t2, 0x100",
        "bnez  t1, 1f",
        "ld    sp, {kernel_sp}(a0)",
        "1:",
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
        kernel_sp = const TrapContext::OFFSET_KERNEL_SP,
        trap_handler = sym trap_handler,
    );
}

#[derive(Clone, Copy, Debug)]
enum PageFaultAccess {
    Execute,
    Read,
    Write,
}

impl PageFaultAccess {
    fn describe(self) -> &'static str {
        match self {
            Self::Execute => "instruction",
            Self::Read => "load",
            Self::Write => "store",
        }
    }
}

fn page_fault_access(exception: Exception) -> PageFaultAccess {
    match exception {
        Exception::InstructionPageFault => PageFaultAccess::Execute,
        Exception::LoadPageFault => PageFaultAccess::Read,
        Exception::StorePageFault => PageFaultAccess::Write,
        _ => unreachable!("not a page fault"),
    }
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

extern "C" fn trap_handler(context: &mut TrapContext) {
    match context.cause() {
        Trap::Interrupt(i) => match Interrupt::from_number(i).unwrap() {
            Interrupt::SupervisorTimer => {
                time::handle_timer_interrupt(context);
                schedule_on_user_return(context);
            }
            Interrupt::SupervisorExternal => match plic::claim_interrupt() {
                Some(UART0_IRQ) => {
                    while let Some(byte) = serial::serial_try_read_byte() {
                        tty::tty_input_byte(byte);
                    }
                    plic::complete_interrupt(UART0_IRQ);
                }
                Some(irq) => {
                    plic::complete_interrupt(irq);
                    #[cfg(feature = "debug_irq")]
                    println_info!(
                        "trap: external interrupt irq={} sepc={:#x} stval={:#x}",
                        irq,
                        context.sepc,
                        context.stval
                    );
                }
                None => {
                    #[cfg(feature = "debug_irq")]
                    println_info!("trap: external interrupt with empty PLIC claim");
                }
            },
            interrupt => {
                #[cfg(feature = "debug_irq")]
                println_info!(
                    "trap: interrupt scause={:#x} interrupt={:?} sepc={:#x} stval={:#x}",
                    context.scause.bits(),
                    interrupt,
                    context.sepc,
                    context.stval
                );
            }
        },
        Trap::Exception(e) => match Exception::from_number(e).unwrap() {
            Exception::Breakpoint => {
                println_info!(
                    "trap: breakpoint scause={:#x} sepc={:#x} stval={:#x}",
                    context.scause.bits(),
                    context.sepc,
                    context.stval
                );
                context.advance_sepc(2);
            }
            Exception::IllegalInstruction => {
                println_fatal!(
                    "trap: illegal instruction scause={:#x} sepc={:#x} stval={:#x} user={}",
                    context.scause.bits(),
                    context.sepc,
                    context.stval,
                    context.is_user()
                );
                halt_forever();
            }
            Exception::UserEnvCall => {
                #[cfg(feature = "debug_syscall")]
                {
                    let syscall_no = context.syscall_no();
                    let syscall_args = context.syscall_args();
                    println_info!(
                        "trap: user ecall no={} args=[{:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}] sepc={:#x}",
                        syscall_no,
                        syscall_args[0],
                        syscall_args[1],
                        syscall_args[2],
                        syscall_args[3],
                        syscall_args[4],
                        syscall_args[5],
                        context.sepc
                    );
                }
                system_call::handle_user_ecall(context);
                schedule_on_user_return(context);
            }
            exception @ (Exception::InstructionPageFault
            | Exception::LoadPageFault
            | Exception::StorePageFault) => {
                let access = page_fault_access(exception);
                println_fatal!(
                    "trap: {} page fault sepc={:#x} stval={:#x} sp={:#x} user={}",
                    access.describe(),
                    context.sepc,
                    context.stval,
                    context.stack_pointer(),
                    context.is_user()
                );
                halt_forever();
            }
            Exception::InstructionMisaligned
            | Exception::LoadMisaligned
            | Exception::StoreMisaligned
            | Exception::InstructionFault
            | Exception::LoadFault
            | Exception::StoreFault => {
                println_fatal!(
                    "trap: bad access exception={:?} scause={:#x} sepc={:#x} stval={:#x} user={}",
                    Exception::from_number(e).unwrap(),
                    context.scause.bits(),
                    context.sepc,
                    context.stval,
                    context.is_user()
                );
                halt_forever();
            }
            exception => {
                println_fatal!(
                    "trap: exception scause={:#x} exception={:?} sepc={:#x} stval={:#x} user={}",
                    context.scause.bits(),
                    exception,
                    context.sepc,
                    context.stval,
                    context.is_user()
                );
                halt_forever();
            }
        },
    }
}
