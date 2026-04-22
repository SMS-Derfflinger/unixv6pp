use crate::{
    constants::PosixError, interrupt::{PtContext, Registers}, interrupt_entry, kernel::diagnose::{diagnose_disable_rows, diagnose_enable_rows, diagnose_rows}, proc::ProcessManager, user::{Pointer, Userspace}
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
    pub const CREAT: usize = 8;
    pub const LINK: usize = 9;
    pub const UNLINK: usize = 10;
    pub const CHDIR: usize = 12;
    pub const TIME: usize = 13;
    pub const MKNOD: usize = 14;
    pub const CHMOD: usize = 15;
    pub const CHOWN: usize = 16;
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
    pub const SYNC: usize = 36;
    pub const GETSWITCH: usize = 38;
    pub const PWD: usize = 39;
    pub const DUP: usize = 41;
    pub const PIPE: usize = 42;
    pub const TIMES: usize = 43;
    pub const PROFIL: usize = 44;
    pub const SETGID: usize = 46;
    pub const GETGID: usize = 47;

    pub fn is_unimplemented(number: usize) -> bool {
        matches!(number, 27 | 33 | 40 | 45 | 49..=63)
    }
}

unsafe extern "C" {
    safe fn cpp_system_call_trap1(number: usize);
}

#[no_mangle]
pub extern "C" fn system_call_body(regs: *mut Registers, context: *mut PtContext) {
    trap(regs, context);
    crate::interrupt::interrupt::schedule_on_user_return(context);
}

fn trap(regs: *mut Registers, context: *mut PtContext) {
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
    Userspace::get().ssav[1] = Pointer(context as usize);
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
        sys::INDIRECT | sys::MOUNT | sys::UMOUNT | sys::PTRACE
            | sys::SMDATE | sys::PROFIL => true,
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
        sys::GETSWITCH => {
            set_eax(ProcessManager::get().switch_cnt);
            true
        }
        sys::PWD => {
            copy_pwd();
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

    times.utime  = Userspace::get().utime;
    times.stime  = Userspace::get().stime;
    times.cutime = Userspace::get().children_utime;
    times.cstime = Userspace::get().children_utime;
}

fn args() -> &'static mut [usize; 5] {
    &mut Userspace::get().args
}

fn set_eax(val: u32) {
    unsafe {
        Userspace::get().ar0.write(val)
    }
}

interrupt_entry!(SystemCallEntrance, system_call_body);
