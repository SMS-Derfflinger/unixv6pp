use crate::{
    constants::PosixError,
    interrupt::{PtContext, PtRegs},
    interrupt_entry,
    user::{Pointer, Userspace},
};

const SYSTEM_CALL_NUM: usize = 64;
const NOERROR: i32 = 0;

unsafe extern "C" {
    safe fn cpp_system_call_arg_count(number: u32) -> u32;
    safe fn cpp_system_call_trap1(number: u32);
}

#[no_mangle]
pub extern "C" fn system_call_body(regs: *mut PtRegs, context: *mut PtContext) {
    trap(regs, context);
    crate::interrupt::interrupt::schedule_on_user_return(context);
}

fn trap(regs: *mut PtRegs, context: *mut PtContext) {
    let Some(regs) = (unsafe { regs.as_mut() }) else {
        return;
    };

    if Userspace::get().proc().should_process() {
        Userspace::get()
            .proc()
            .process_signal(unsafe { &mut *(context as *mut _) });
        Userspace::get().error = Some(PosixError::EINTR);
        regs.eax = -(PosixError::EINTR as i32) as u32;
        return;
    }

    Userspace::get().ssav[0] = Pointer((&raw mut *regs) as usize);
    Userspace::get().ssav[1] = Pointer(context as usize);
    Userspace::get().ar0 = &raw mut regs.eax;
    Userspace::get().error = None;

    let number = regs.eax;
    let count = if (number as usize) < SYSTEM_CALL_NUM {
        cpp_system_call_arg_count(number)
    } else {
        0
    };
    copy_args(regs, context, count as usize);

    cpp_system_call_trap1(number);

    if Userspace::get().signal_pending {
        Userspace::get().error = Some(PosixError::EINTR);
        regs.eax = -(PosixError::EINTR as i32) as u32;
    }

    if let Some(err) = Userspace::get().error {
        regs.eax = -(err as i32) as u32;
        crate::println_info!("regs->eax={:#x} error={err:?}", regs.eax);
    }

    if Userspace::get().proc().should_process() {
        Userspace::get()
            .proc()
            .process_signal(unsafe { &mut *(context as *mut _) });
    }

    Userspace::get().proc().set_pri();
}

fn copy_args(regs: &PtRegs, context: *mut PtContext, count: usize) {
    let args = &mut Userspace::get().args;
    let syscall_args = [regs.ebx, regs.ecx, regs.edx, regs.esi, regs.edi];

    for (argref, arg) in args.iter_mut().zip(syscall_args).take(count) {
        *argref = arg as usize;
    }

    Userspace::get().dirp = args[0] as *mut u8;
}

interrupt_entry!(SystemCallEntrance, system_call_body);
