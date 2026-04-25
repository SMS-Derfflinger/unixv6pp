use core::ptr::NonNull;

use crate::{
    constants::{PosixError, Signal, PSLEP},
    interrupt::{
        time::{get_time, time_set_tout, time_tout, time_tout_address},
        Registers,
    },
    interrupt_entry,
    kernel::diagnose::{diagnose_disable_rows, diagnose_enable_rows, diagnose_rows},
    machine::{asm::disable_interrupts, TrapFrame},
    proc::{ProcessManager, TaskContext},
    sync::IrqGuard,
    user::{Pointer, Userspace},
};

const SYSTEM_CALL_NUM: usize = 64;

mod syscall_number {
    pub const INDIRECT: usize = 0;
    pub const EXIT: usize = 1;
    pub const FORK: usize = 2;
    pub const READ: usize = 3;
    pub const WRITE: usize = 4;
    pub const OPEN: usize = 5;
    pub const CLOSE: usize = 6;
    pub const WAIT: usize = 7;
    pub const CREAT: usize = 8;
    pub const LINK: usize = 9;
    pub const UNLINK: usize = 10;
    pub const EXEC: usize = 11;
    pub const CHDIR: usize = 12;
    pub const TIME: usize = 13;
    pub const MKNOD: usize = 14;
    pub const CHMOD: usize = 15;
    pub const CHOWN: usize = 16;
    pub const SBREAK: usize = 17;
    pub const STAT: usize = 18;
    pub const SEEK: usize = 19;
    pub const GETPID: usize = 20;
    pub const MOUNT: usize = 21;
    pub const UMOUNT: usize = 22;
    pub const SETUID: usize = 23;
    pub const GETUID: usize = 24;
    pub const STIME: usize = 25;
    pub const PTRACE: usize = 26;
    pub const FSTAT: usize = 28;
    pub const SMDATE: usize = 30;
    pub const TRACE: usize = 29;
    pub const NICE: usize = 34;
    pub const SLEEP: usize = 35;
    pub const SYNC: usize = 36;
    pub const KILL: usize = 37;
    pub const GETSWITCH: usize = 38;
    pub const PWD: usize = 39;
    pub const DUP: usize = 41;
    pub const PIPE: usize = 42;
    pub const TIMES: usize = 43;
    pub const PROFIL: usize = 44;
    pub const SETGID: usize = 46;
    pub const GETGID: usize = 47;
    pub const SSIG: usize = 48;

    pub fn is_unimplemented(number: usize) -> bool {
        matches!(number, 27 | 33 | 40 | 45 | 49..=63)
    }
}

unsafe extern "C" {
    safe fn cpp_system_call_trap1(number: usize);
}

#[no_mangle]
pub extern "C" fn system_call_body(regs: *mut Registers, context: &mut TrapFrame) {
    trap(regs, context);
    crate::interrupt::interrupt::schedule_on_user_return(context);
}

fn trap(regs: *mut Registers, context: &mut TrapFrame) {
    let Some(regs) = (unsafe { regs.as_mut() }) else {
        return;
    };

    if Userspace::get().proc().should_process() {
        Userspace::get()
            .proc()
            .process_signal(unsafe { &mut *(context as *mut _) });
        Userspace::get().error = Some(PosixError::EINTR);
        regs.eax = -(PosixError::EINTR as i32) as usize;
        return;
    }

    Userspace::get().ssav[0] = Pointer((&raw mut *regs) as usize);
    Userspace::get().ssav[1] = Pointer(context as *mut _ as usize);
    Userspace::get().ar0 = &raw mut regs.eax as *mut _;
    Userspace::get().error = None;

    let syscall_no = regs.eax;
    copy_args(regs);

    if !handle_in_rust(syscall_no) {
        cpp_system_call_trap1(syscall_no);
    }

    if Userspace::get().signal_pending {
        Userspace::get().error = Some(PosixError::EINTR);
        regs.eax = -(PosixError::EINTR as i32) as usize;
    }

    if let Some(err) = Userspace::get().error {
        regs.eax = -(err as i32) as usize;
        crate::println_info!("regs->eax={:#x} error={err:?}", regs.eax);
    }

    if Userspace::get().proc().should_process() {
        Userspace::get()
            .proc()
            .process_signal(unsafe { &mut *(context as *mut _) });
    }

    Userspace::get().proc().set_pri();
}

fn copy_args(regs: &Registers) {
    let args = &mut Userspace::get().args;
    let syscall_args = [regs.ebx, regs.ecx, regs.edx, regs.esi, regs.edi];

    for (argref, arg) in args.iter_mut().zip(syscall_args) {
        *argref = arg as usize;
    }

    Userspace::get().dirp = args[0] as *mut u8;
}

fn trap1(handler: fn()) {
    Userspace::get().signal_pending = true;
    handler();
    Userspace::get().signal_pending = false;
}

fn handle_in_rust(number: usize) -> bool {
    use syscall_number as sys;

    match number {
        sys::INDIRECT | sys::MOUNT | sys::UMOUNT | sys::PTRACE | sys::SMDATE | sys::PROFIL => true,
        sys::EXIT => Userspace::get().proc().exit(),
        sys::FORK => {
            set_eax(ProcessManager::get().fork());
            true
        }
        sys::READ => {
            trap1(crate::fs::syscall_read);
            true
        }
        sys::WRITE => {
            trap1(crate::fs::syscall_write);
            true
        }
        sys::OPEN => {
            trap1(crate::fs::syscall_open);
            true
        }
        sys::CLOSE => {
            trap1(crate::fs::syscall_close);
            true
        }
        sys::WAIT => {
            ProcessManager::get().wait(Userspace::get().proc());
            true
        }
        sys::CREAT => {
            trap1(crate::fs::syscall_creat);
            true
        }
        sys::LINK => {
            trap1(crate::fs::syscall_link);
            true
        }
        sys::UNLINK => {
            trap1(crate::fs::syscall_unlink);
            true
        }
        sys::EXEC => {
            let pm = ProcessManager::get();
            let proc = Userspace::get().proc();
            let path_ptr = Userspace::get().args[0] as *const u8;
            let argc = Userspace::get().args[1];
            let argv_ptr = Userspace::get().args[2] as *const NonNull<i8>;

            // 从用户空间读取路径
            let path = unsafe {
                let cstr = core::ffi::CStr::from_ptr(path_ptr as *const i8);
                cstr.to_bytes()
            };

            let argv = unsafe { core::slice::from_raw_parts(argv_ptr, argc) };

            crate::println_info!("Execing: {}", core::str::from_utf8(path).unwrap());
            if let Err(err) = pm.exec(proc, path, argv) {
                Userspace::get().set_error(err);
                return true;
            }

            let mut ctx = TaskContext::new();
            unsafe {
                disable_interrupts();
                TaskContext::switch(&mut ctx, &mut proc.ctx);
            }

            true
        }
        sys::CHDIR => {
            trap1(crate::fs::syscall_chdir);
            true
        }
        sys::TIME => {
            set_eax(super::time::get_time());
            true
        }
        sys::MKNOD => {
            trap1(crate::fs::syscall_mknod);
            true
        }
        sys::CHMOD => {
            trap1(crate::fs::syscall_chmod);
            true
        }
        sys::CHOWN => {
            trap1(crate::fs::syscall_chown);
            true
        }
        sys::SBREAK => {
            let brk = Userspace::get().args[0];
            match Userspace::get().proc().sbrk(brk) {
                Ok(retval) => Userspace::get().set_user_retval(retval as u32),
                Err(err) => Userspace::get().set_error(err),
            }
            true
        }
        sys::STAT => {
            trap1(crate::fs::syscall_stat);
            true
        }
        sys::SEEK => {
            trap1(crate::fs::syscall_seek);
            true
        }
        sys::GETPID => {
            set_eax(Userspace::get().proc().pid);
            true
        }
        sys::SETUID => {
            let uid = Userspace::get().args[0];
            Userspace::get().setuid(uid as _, uid as _);
            true
        }
        sys::GETUID => {
            Userspace::get().set_user_retval(Userspace::get().getuid());
            true
        }
        sys::STIME => {
            if Userspace::get().is_root() {
                super::time::time_set(args()[0] as u32);
            }
            true
        }
        number if sys::is_unimplemented(number) => {
            Userspace::get().error = Some(PosixError::ENOSYS);
            true
        }
        sys::FSTAT => {
            trap1(crate::fs::syscall_fstat);
            true
        }
        sys::TRACE => {
            if diagnose_rows() == 0 {
                diagnose_enable_rows(args()[0] as u32);
            } else {
                diagnose_disable_rows();
            }
            set_eax(diagnose_rows());
            true
        }
        sys::KILL => {
            let pid = Userspace::get().args[0] as u32;
            let signal_num = Userspace::get().args[1] as u32;
            let signal = unsafe { core::mem::transmute::<u32, Signal>(signal_num) };
            match ProcessManager::get().kill(pid, signal) {
                Ok(()) => Userspace::get().set_user_retval(0),
                Err(err) => Userspace::get().set_error(err),
            }

            true
        }
        sys::GETSWITCH => {
            set_eax(ProcessManager::get().switch_cnt);
            true
        }
        sys::PWD => {
            copy_pwd();
            true
        }
        sys::NICE => {
            let nice = Userspace::get().args[0] as i32;
            Userspace::get().proc().set_nice(nice);
            true
        }
        sys::SLEEP => {
            let _ctx = IrqGuard::disable_save();
            let wake_time = get_time() + Userspace::get().args[0] as u32;

            loop {
                let now = get_time();
                let tout = time_tout();

                if wake_time <= now {
                    break;
                }

                if tout <= now || tout > wake_time {
                    time_set_tout(wake_time);
                }

                Userspace::get()
                    .proc()
                    .sleep_user(time_tout_address(), PSLEP);
            }

            true
        }
        sys::SYNC => {
            trap1(crate::fs::syscall_sync);
            true
        }
        sys::DUP => {
            trap1(crate::fs::syscall_dup);
            true
        }
        sys::PIPE => {
            trap1(crate::fs::syscall_pipe);
            true
        }
        sys::TIMES => {
            write_times();
            true
        }
        sys::SETGID => {
            let gid = Userspace::get().args[0];
            Userspace::get().setgid(gid as _, gid as _);
            true
        }
        sys::GETGID => {
            Userspace::get().set_user_retval(Userspace::get().getgid());
            true
        }
        sys::SSIG => {
            let signal = Userspace::get().args[0] as u32;
            let func = Userspace::get().args[1];
            match Userspace::get().proc().send_signal(signal, func) {
                Ok(retval) => Userspace::get().set_user_retval(retval as u32),
                Err(err) => Userspace::get().set_error(err),
            }
            true
        }
        _ => false,
    }
}

fn copy_pwd() {
    let mut dst = Userspace::get().dirp;
    if dst.is_null() {
        return;
    }

    for &ch in &Userspace::get().cwd_full {
        unsafe { dst.write(ch) };
        dst = dst.wrapping_add(1);
        if ch == 0 {
            break;
        }
    }
}

#[repr(C)]
struct Tms {
    utime: u32,
    stime: u32,
    cutime: u32,
    cstime: u32,
}

fn write_times() {
    let ptr = args()[0] as *mut Tms;
    let Some(times) = (unsafe { ptr.as_mut() }) else {
        return;
    };

    times.utime = Userspace::get().utime;
    times.stime = Userspace::get().stime;
    times.cutime = Userspace::get().children_utime;
    times.cstime = Userspace::get().children_utime;
}

fn args() -> &'static mut [usize; 5] {
    &mut Userspace::get().args
}

fn set_eax(val: u32) {
    unsafe { Userspace::get().ar0.write(val) }
}

interrupt_entry!(SystemCallEntrance, system_call_body);
