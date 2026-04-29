use core::ptr::NonNull;

use crate::{
    constants::{PosixError, Signal, PSLEP},
    interrupt::{
        context::TrapContext,
        schedule_on_user_return,
        time::{get_time, sleep_user_until},
    },
    proc::ProcessManager,
    serial::KResult,
    user::{Pointer, Userspace},
};

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
    pub const TRACE: usize = 29;
    pub const SMDATE: usize = 30;
    pub const STTY: usize = 31;
    pub const GTTY: usize = 32;
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

pub fn handle_user_ecall(context: &mut TrapContext) {
    if Userspace::get().proc().should_process() {
        Userspace::get().proc().process_signal(context);
        Userspace::get().set_error(PosixError::EINTR);
        context.set_return_value((-(PosixError::EINTR as isize)) as usize);
        return;
    }

    context.advance_sepc(4);
    Userspace::get().ssav[1] = Pointer(context as *mut _ as usize);
    Userspace::get().ar0 = &raw mut context.regs.a0;
    Userspace::get().error = None;
    copy_args(context);

    let syscall_no = context.syscall_no();
    let mut syscall_result = handle_in_rust(syscall_no);

    if Userspace::get().signal_pending {
        syscall_result = Err(PosixError::EINTR);
    }

    match syscall_result {
        Err(err) => context.set_return_value((-(err as isize)) as usize),
        Ok(retval) => context.set_return_value(retval),
    }

    if Userspace::get().proc().should_process() {
        Userspace::get().proc().process_signal(context);
    }

    Userspace::get().proc().set_pri();

    schedule_on_user_return(context);
}

fn copy_args(context: &TrapContext) {
    let args = context.syscall_args();
    Userspace::get().args = args;
    Userspace::get().dirp = args[0] as *mut u8;
}

fn trap_ret(handler: fn()) -> KResult<usize> {
    handler();
    match Userspace::get().error {
        Some(err) => Err(err),
        None => Ok(unsafe { Userspace::get().ar0.read() as usize }),
    }
}

fn trap_void(handler: fn()) -> KResult<usize> {
    handler();
    match Userspace::get().error {
        Some(err) => Err(err),
        None => Ok(0),
    }
}

fn handle_in_rust(number: usize) -> KResult<usize> {
    use syscall_number as sys;

    match number {
        sys::INDIRECT | sys::MOUNT | sys::UMOUNT | sys::PTRACE | sys::SMDATE | sys::PROFIL => Ok(0),
        sys::EXIT => Userspace::get().proc().exit(),
        sys::FORK => Ok(ProcessManager::get().fork() as usize),
        sys::READ => trap_ret(crate::fs::syscall_read),
        sys::WRITE => trap_ret(crate::fs::syscall_write),
        sys::OPEN => trap_ret(crate::fs::syscall_open),
        sys::CLOSE => trap_void(crate::fs::syscall_close),
        sys::WAIT => ProcessManager::get()
            .wait(Userspace::get().proc())
            .map(|pid| pid as usize),
        sys::CREAT => trap_ret(crate::fs::syscall_creat),
        sys::LINK => trap_void(crate::fs::syscall_link),
        sys::UNLINK => trap_void(crate::fs::syscall_unlink),
        sys::EXEC => {
            let pm = ProcessManager::get();
            let proc = Userspace::get().proc();
            let path_ptr = Userspace::get().args[0] as *const u8;
            let argc = Userspace::get().args[1];
            let argv_ptr = Userspace::get().args[2] as *const NonNull<i8>;
            let path = unsafe { core::ffi::CStr::from_ptr(path_ptr).to_bytes() };
            let argv = unsafe { core::slice::from_raw_parts(argv_ptr, argc) };

            crate::println_info!("Execing: {}", core::str::from_utf8(path).unwrap());
            pm.exec(proc, path, argv)?;

            Ok(0)
        }
        sys::CHDIR => trap_void(crate::fs::syscall_chdir),
        sys::TIME => Ok(super::time::get_time() as usize),
        sys::MKNOD => trap_void(crate::fs::syscall_mknod),
        sys::CHMOD => trap_void(crate::fs::syscall_chmod),
        sys::CHOWN => trap_void(crate::fs::syscall_chown),
        sys::SBREAK => Userspace::get().proc().sbrk(Userspace::get().args[0]),
        sys::STAT => trap_void(crate::fs::syscall_stat),
        sys::SEEK => trap_void(crate::fs::syscall_seek),
        sys::GETPID => Ok(Userspace::get().proc().pid as usize),
        sys::SETUID => {
            let uid = Userspace::get().args[0];
            Userspace::get().setuid(uid as _, uid as _);
            Ok(0)
        }
        sys::GETUID => Ok(Userspace::get().getuid() as usize),
        sys::STIME => {
            if !Userspace::get().is_root() {
                return Err(PosixError::EPERM);
            }

            crate::println_warn!("stime is not implemented on riscv64");
            Ok(0)
        }
        number if sys::is_unimplemented(number) => Err(PosixError::ENOSYS),
        sys::FSTAT => trap_void(crate::fs::syscall_fstat),
        sys::TRACE => Err(PosixError::ENOSYS),
        sys::KILL => {
            let pid = Userspace::get().args[0] as u32;
            let signal_num = Userspace::get().args[1] as u32;
            let signal = unsafe { core::mem::transmute::<u32, Signal>(signal_num) };
            ProcessManager::get().kill(pid, signal).map(|()| 0)
        }
        sys::GETSWITCH => Ok(ProcessManager::get().switch_cnt as usize),
        sys::PWD => {
            copy_pwd();
            Ok(0)
        }
        // TODO: stty and gtty are blank implementations for now.
        sys::STTY | sys::GTTY => Ok(0),
        sys::NICE => {
            let nice = Userspace::get().args[0] as i32;
            Userspace::get().proc().set_nice(nice);
            Ok(0)
        }
        sys::SLEEP => {
            let sleep_time = Userspace::get().args[0];
            let wake_time = get_time() + sleep_time;

            sleep_user_until(wake_time, PSLEP)?;
            Ok(0)
        }
        sys::SYNC => trap_void(crate::fs::syscall_sync),
        sys::DUP => trap_ret(crate::fs::syscall_dup),
        sys::PIPE => trap_void(crate::fs::syscall_pipe),
        sys::TIMES => {
            write_times();
            Ok(0)
        }
        sys::SETGID => {
            let gid = Userspace::get().args[0];
            Userspace::get().setgid(gid as _, gid as _);
            Ok(0)
        }
        sys::GETGID => Ok(Userspace::get().getgid() as usize),
        sys::SSIG => {
            let signal = Userspace::get().args[0] as u32;
            let func = Userspace::get().args[1];
            Userspace::get()
                .proc()
                .send_signal(signal, func)
                .map(|retval| retval as usize)
        }
        _ => Err(PosixError::ENOSYS),
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

fn args() -> &'static mut [usize; 6] {
    &mut Userspace::get().args
}
