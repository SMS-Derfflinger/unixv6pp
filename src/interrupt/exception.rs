use crate::{
    constants::Signal,
    interrupt::Registers,
    interrupt_entry, interrupt_entry_with_error_code,
    machine::{TrapFrame, TrapFrameWithError},
    user::Userspace,
};

fn handle_exception(context: &mut TrapFrame, signal: Option<Signal>, message: &str) {
    if context.is_user() {
        panic!("Unhandled kernel space exception: {}", message);
    }

    if let Some(signal) = signal {
        Userspace::get().proc().raise(signal);
    }

    if Userspace::get().proc().should_process() {
        Userspace::get().proc().process_signal(context);
    }
}

fn handle_exception_with_error_code(
    context: &mut TrapFrameWithError,
    signal: Option<Signal>,
    message: &str,
) {
    handle_exception(&mut *context, signal, message);
}

#[no_mangle]
pub extern "C" fn divide_error(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGFPE), "Divide Exception!");
}

#[no_mangle]
pub extern "C" fn debug(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGTRAP), "Debug Exception!");
}

#[no_mangle]
pub extern "C" fn nmi(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, None, "Non-maskable Interrupt!");
}

#[no_mangle]
pub extern "C" fn breakpoint(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGTRAP), "Breakpoint Exception!");
}

#[no_mangle]
pub extern "C" fn overflow(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGSEGV), "Overflow Exception!");
}

#[no_mangle]
pub extern "C" fn bound(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGSEGV), "Bound Range Exceeded!");
}

#[no_mangle]
pub extern "C" fn invalid_opcode(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGILL), "Invalid Opcode!");
}

#[no_mangle]
pub extern "C" fn device_not_available(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGSEGV), "Device Not Available!");
}

#[no_mangle]
pub extern "C" fn double_fault(_regs: *mut Registers, context: &mut TrapFrameWithError) {
    handle_exception_with_error_code(context, Some(Signal::SIGSEGV), "Double Fault Exception!");
}

#[no_mangle]
pub extern "C" fn coprocessor_segment_overrun(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(
        context,
        Some(Signal::SIGFPE),
        "Coprocessor Segment Overrun!",
    );
}

#[no_mangle]
pub extern "C" fn invalid_tss(_regs: *mut Registers, context: &mut TrapFrameWithError) {
    handle_exception_with_error_code(context, Some(Signal::SIGSEGV), "Invalid TSS!");
}

#[no_mangle]
pub extern "C" fn segment_not_present(_regs: *mut Registers, context: &mut TrapFrameWithError) {
    handle_exception_with_error_code(context, Some(Signal::SIGBUS), "Segment Not Present!");
}

#[no_mangle]
pub extern "C" fn stack_segment_error(_regs: *mut Registers, context: &mut TrapFrameWithError) {
    handle_exception_with_error_code(context, Some(Signal::SIGBUS), "Stack Segment Error!");
}

#[no_mangle]
pub extern "C" fn general_protection(_regs: *mut Registers, context: &mut TrapFrameWithError) {
    handle_exception_with_error_code(context, Some(Signal::SIGSEGV), "General Protection!");
}

#[no_mangle]
pub extern "C" fn page_fault(_regs: *mut Registers, context: &mut TrapFrameWithError) {
    let fault_addr: usize;
    unsafe {
        core::arch::asm!(
            "mov %cr2, {}",
            out(reg) fault_addr,
            options(att_syntax),
        );
    }

    if context.is_user() {
        panic!(
            "Kernel space page fault at pc={:#x} addr={:#x}",
            context.eip, fault_addr
        );
    }

    let mem = &mut Userspace::get().mem;
    const USER_SPACE_SIZE: usize = 0x800000;
    if fault_addr < USER_SPACE_SIZE && fault_addr >= 0x600000 && !mem.overflow() {
        crate::println_debug!("Stack size enlarged");
        Userspace::get().proc().sstack();
        return;
    }

    crate::println_warn!(
        "Segmentation fault at pc={:#x} addr={:#x}",
        context.eip,
        fault_addr
    );

    Userspace::get().proc().raise(Signal::SIGSEGV);
    if Userspace::get().proc().should_process() {
        Userspace::get().proc().process_signal(context);
    }
}

#[no_mangle]
pub extern "C" fn coprocessor_error(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGFPE), "Coprocessor Error!");
}

#[no_mangle]
pub extern "C" fn alignment_check(_regs: *mut Registers, context: &mut TrapFrameWithError) {
    handle_exception_with_error_code(context, Some(Signal::SIGBUS), "Alignment Check!");
}

#[no_mangle]
pub extern "C" fn machine_check(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, None, "Machine Check!");
}

#[no_mangle]
pub extern "C" fn simd_exception(_regs: *mut Registers, context: &mut TrapFrame) {
    handle_exception(context, Some(Signal::SIGFPE), "SIMD Float Point Exception!");
}

interrupt_entry!(DivideErrorEntrance, divide_error);
interrupt_entry!(DebugEntrance, debug);
interrupt_entry!(NMIEntrance, nmi);
interrupt_entry!(BreakpointEntrance, breakpoint);
interrupt_entry!(OverflowEntrance, overflow);
interrupt_entry!(BoundEntrance, bound);
interrupt_entry!(InvalidOpcodeEntrance, invalid_opcode);
interrupt_entry!(DeviceNotAvailableEntrance, device_not_available);
interrupt_entry_with_error_code!(DoubleFaultEntrance, double_fault);
interrupt_entry!(
    CoprocessorSegmentOverrunEntrance,
    coprocessor_segment_overrun
);
interrupt_entry_with_error_code!(InvalidTSSEntrance, invalid_tss);
interrupt_entry_with_error_code!(SegmentNotPresentEntrance, segment_not_present);
interrupt_entry_with_error_code!(StackSegmentErrorEntrance, stack_segment_error);
interrupt_entry_with_error_code!(GeneralProtectionEntrance, general_protection);
interrupt_entry_with_error_code!(PageFaultEntrance, page_fault);
interrupt_entry!(CoprocessorErrorEntrance, coprocessor_error);
interrupt_entry_with_error_code!(AlignmentCheckEntrance, alignment_check);
interrupt_entry!(MachineCheckEntrance, machine_check);
interrupt_entry!(SIMDExceptionEntrance, simd_exception);
