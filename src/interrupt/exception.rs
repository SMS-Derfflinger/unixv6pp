use crate::{
    interrupt::{PtContext, Registers, PteContext},
    interrupt_entry, interrupt_entry_with_error_code,
};

const SIGNUL: i32 = 0;
const SIGILL: i32 = 4;
const SIGTRAP: i32 = 5;
const SIGBUS: i32 = 7;
const SIGFPE: i32 = 8;
const SIGSEGV: i32 = 11;

unsafe extern "C" {
    safe fn cpp_exception_handle(context: *mut PtContext, signal: i32, message: *const u8);
    safe fn cpp_exception_page_fault(regs: *mut Registers, context: *mut PteContext);
}

fn handle_exception(context: *mut PtContext, signal: i32, message: &'static [u8]) {
    cpp_exception_handle(context, signal, message.as_ptr());
}

fn handle_exception_with_error_code(context: *mut PteContext, signal: i32, message: &'static [u8]) {
    let Some(context) = (unsafe { context.as_mut() }) else {
        return;
    };

    cpp_exception_handle(context.as_context(), signal, message.as_ptr());
}

#[no_mangle]
pub extern "C" fn divide_error(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGFPE, b"Divide Exception!\0");
}

#[no_mangle]
pub extern "C" fn debug(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGTRAP, b"Debug Exception!\0");
}

#[no_mangle]
pub extern "C" fn nmi(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGNUL, b"Non-maskable Interrupt!\0");
}

#[no_mangle]
pub extern "C" fn breakpoint(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGTRAP, b"Breakpoint Exception!\0");
}

#[no_mangle]
pub extern "C" fn overflow(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGSEGV, b"Overflow Exception!\0");
}

#[no_mangle]
pub extern "C" fn bound(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGSEGV, b"Bound Range Exceeded!\0");
}

#[no_mangle]
pub extern "C" fn invalid_opcode(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGILL, b"Invalid Opcode!\0");
}

#[no_mangle]
pub extern "C" fn device_not_available(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGSEGV, b"Device Not Available!\0");
}

#[no_mangle]
pub extern "C" fn double_fault(_regs: *mut Registers, context: *mut PteContext) {
    handle_exception_with_error_code(context, SIGSEGV, b"Double Fault Exception!\0");
}

#[no_mangle]
pub extern "C" fn coprocessor_segment_overrun(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGFPE, b"Coprocessor Segment Overrun!\0");
}

#[no_mangle]
pub extern "C" fn invalid_tss(_regs: *mut Registers, context: *mut PteContext) {
    handle_exception_with_error_code(context, SIGSEGV, b"Invalid TSS!\0");
}

#[no_mangle]
pub extern "C" fn segment_not_present(_regs: *mut Registers, context: *mut PteContext) {
    handle_exception_with_error_code(context, SIGBUS, b"Segment Not Present!\0");
}

#[no_mangle]
pub extern "C" fn stack_segment_error(_regs: *mut Registers, context: *mut PteContext) {
    handle_exception_with_error_code(context, SIGBUS, b"Stack Segment Error!\0");
}

#[no_mangle]
pub extern "C" fn general_protection(_regs: *mut Registers, context: *mut PteContext) {
    handle_exception_with_error_code(context, SIGSEGV, b"General Protection!\0");
}

#[no_mangle]
pub extern "C" fn page_fault(regs: *mut Registers, context: *mut PteContext) {
    cpp_exception_page_fault(regs, context);
}

#[no_mangle]
pub extern "C" fn coprocessor_error(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGFPE, b"Coprocessor Error!\0");
}

#[no_mangle]
pub extern "C" fn alignment_check(_regs: *mut Registers, context: *mut PteContext) {
    handle_exception_with_error_code(context, SIGBUS, b"Alignment Check!\0");
}

#[no_mangle]
pub extern "C" fn machine_check(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGNUL, b"Machine Check!\0");
}

#[no_mangle]
pub extern "C" fn simd_exception(_regs: *mut Registers, context: *mut PtContext) {
    handle_exception(context, SIGFPE, b"SIMD Float Point Exception!\0");
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
interrupt_entry!(CoprocessorSegmentOverrunEntrance, coprocessor_segment_overrun);
interrupt_entry_with_error_code!(InvalidTSSEntrance, invalid_tss);
interrupt_entry_with_error_code!(SegmentNotPresentEntrance, segment_not_present);
interrupt_entry_with_error_code!(StackSegmentErrorEntrance, stack_segment_error);
interrupt_entry_with_error_code!(GeneralProtectionEntrance, general_protection);
interrupt_entry_with_error_code!(PageFaultEntrance, page_fault);
interrupt_entry!(CoprocessorErrorEntrance, coprocessor_error);
interrupt_entry_with_error_code!(AlignmentCheckEntrance, alignment_check);
interrupt_entry!(MachineCheckEntrance, machine_check);
interrupt_entry!(SIMDExceptionEntrance, simd_exception);
