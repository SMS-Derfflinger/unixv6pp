use core::{arch::asm, ffi::CStr};

const SYS_EXIT: usize = 1;
const SYS_FORK: usize = 2;
const SYS_READ: usize = 3;
const SYS_WRITE: usize = 4;
const SYS_OPEN: usize = 5;
const SYS_CLOSE: usize = 6;
const SYS_WAIT: usize = 7;
const SYS_CREAT: usize = 8;
const SYS_LINK: usize = 9;
const SYS_UNLINK: usize = 10;
const SYS_EXECV: usize = 11;
const SYS_CHDIR: usize = 12;
const SYS_MKNOD: usize = 14;
const SYS_CHMOD: usize = 15;
const SYS_CHOWN: usize = 16;
const SYS_BRK: usize = 17;
const SYS_STAT: usize = 18;
const SYS_SEEK: usize = 19;
const SYS_GETPID: usize = 20;
const SYS_SETUID: usize = 23;
const SYS_GETUID: usize = 24;
const SYS_FSTAT: usize = 28;
const SYS_NICE: usize = 34;
const SYS_SLEEP: usize = 35;
const SYS_SYNC_FILE_SYSTEM: usize = 36;
const SYS_KILL: usize = 37;
const SYS_PWD: usize = 39;
const SYS_DUP: usize = 41;
const SYS_PIPE: usize = 42;
const SYS_SETGID: usize = 46;
const SYS_GETGID: usize = 47;
const SYS_SIGNAL: usize = 48;

#[no_mangle]
pub extern "C" fn _lib_creat(pathname: *mut u8, mode: u32) -> i32 {
    normalize(syscall2(SYS_CREAT, pathname as usize, mode as usize))
}

#[no_mangle]
pub extern "C" fn _lib_open(pathname: &CStr, mode: u32) -> i32 {
    normalize(syscall2(
        SYS_OPEN,
        pathname.as_ptr() as usize,
        mode as usize,
    ))
}

#[no_mangle]
pub extern "C" fn _lib_close(fd: i32) -> i32 {
    normalize(syscall1(SYS_CLOSE, fd as usize))
}

#[no_mangle]
pub extern "C" fn _lib_read(fd: i32, buf: *mut u8, nbytes: i32) -> i32 {
    normalize(syscall3(
        SYS_READ,
        fd as usize,
        buf as usize,
        nbytes as usize,
    ))
}

#[no_mangle]
pub extern "C" fn _lib_write(fd: i32, buf: *mut u8, nbytes: i32) -> i32 {
    normalize(syscall3(
        SYS_WRITE,
        fd as usize,
        buf as usize,
        nbytes as usize,
    ))
}

#[no_mangle]
pub extern "C" fn _lib_exit(status: i32) -> i32 {
    normalize(syscall1(SYS_EXIT, status as usize))
}

#[no_mangle]
pub extern "C" fn _lib_wait(status: *mut i32) -> i32 {
    normalize(syscall1(SYS_WAIT, status as usize))
}

#[no_mangle]
pub extern "C" fn _lib_fork() -> i32 {
    normalize(syscall0(SYS_FORK))
}

#[no_mangle]
pub extern "C" fn _lib_pipe(fildes: *mut i32) -> i32 {
    normalize(syscall1(SYS_PIPE, fildes as usize))
}

#[no_mangle]
pub extern "C" fn _lib_execv(pathname: *mut u8, argv: *mut *mut u8) -> i32 {
    normalize(syscall3(SYS_EXECV, pathname as usize, 0, argv as usize))
}

#[no_mangle]
pub extern "C" fn _lib_seek(fd: i32, offset: u32, ptrname: u32) -> i32 {
    normalize(syscall3(
        SYS_SEEK,
        fd as usize,
        offset as usize,
        ptrname as usize,
    ))
}

#[no_mangle]
pub extern "C" fn _lib_dup(fd: i32) -> i32 {
    normalize(syscall1(SYS_DUP, fd as usize))
}

#[no_mangle]
pub extern "C" fn _lib_fstat(fd: i32, statbuf: usize) -> i32 {
    normalize(syscall2(SYS_FSTAT, fd as usize, statbuf))
}

#[no_mangle]
pub extern "C" fn _lib_stat(pathname: *mut u8, statbuf: usize) -> i32 {
    normalize(syscall2(SYS_STAT, pathname as usize, statbuf))
}

#[no_mangle]
pub extern "C" fn _lib_chmod(pathname: *mut u8, mode: u32) -> i32 {
    normalize(syscall2(SYS_CHMOD, pathname as usize, mode as usize))
}

#[no_mangle]
pub extern "C" fn _lib_chown(pathname: *mut u8, uid: i16, gid: i16) -> i32 {
    normalize(syscall3(
        SYS_CHOWN,
        pathname as usize,
        uid as usize,
        gid as usize,
    ))
}

#[no_mangle]
pub extern "C" fn _lib_getuid() -> i16 {
    normalize_short(syscall0(SYS_GETUID))
}

#[no_mangle]
pub extern "C" fn _lib_setuid(uid: i16) -> i32 {
    normalize(syscall1(SYS_SETUID, uid as usize))
}

#[no_mangle]
pub extern "C" fn _lib_getgid() -> i16 {
    normalize_short(syscall0(SYS_GETGID))
}

#[no_mangle]
pub extern "C" fn _lib_setgid(gid: i16) -> i32 {
    normalize(syscall1(SYS_SETGID, gid as usize))
}

#[no_mangle]
pub extern "C" fn _lib_getpid() -> i32 {
    normalize(syscall0(SYS_GETPID))
}

#[no_mangle]
pub extern "C" fn _lib_nice(change: i32) -> i32 {
    normalize(syscall1(SYS_NICE, change as usize))
}

#[no_mangle]
pub extern "C" fn _lib_sig(signal: i32, func: usize) -> i32 {
    normalize(syscall2(SYS_SIGNAL, signal as usize, func))
}

#[no_mangle]
pub extern "C" fn _lib_kill(pid: i32, signal: i32) -> i32 {
    normalize(syscall2(SYS_KILL, pid as usize, signal as usize))
}

#[no_mangle]
pub extern "C" fn _lib_sleep(seconds: u32) -> i32 {
    normalize(syscall1(SYS_SLEEP, seconds as usize))
}

#[no_mangle]
pub extern "C" fn _lib_pwd(pwd: *mut u8) -> i32 {
    normalize(syscall1(SYS_PWD, pwd as usize))
}

#[no_mangle]
pub extern "C" fn _lib_brk(new_size: u32) -> i32 {
    normalize(syscall1(SYS_BRK, new_size as usize))
}

#[no_mangle]
pub extern "C" fn _lib_link(pathname: *mut u8, new_pathname: *mut u8) -> i32 {
    normalize(syscall2(SYS_LINK, pathname as usize, new_pathname as usize))
}

#[no_mangle]
pub extern "C" fn _lib_unlink(pathname: *mut u8) -> i32 {
    normalize(syscall1(SYS_UNLINK, pathname as usize))
}

#[no_mangle]
pub extern "C" fn _lib_chdir(pathname: *mut u8) -> i32 {
    normalize(syscall1(SYS_CHDIR, pathname as usize))
}

#[no_mangle]
pub extern "C" fn _lib_mknod(pathname: *mut u8, mode: u32, dev: i32) -> i32 {
    normalize(syscall3(
        SYS_MKNOD,
        pathname as usize,
        mode as usize,
        dev as usize,
    ))
}

#[no_mangle]
pub extern "C" fn _lib_sync_file_system() -> i32 {
    normalize(syscall0(SYS_SYNC_FILE_SYSTEM))
}

fn normalize(result: i32) -> i32 {
    if result >= 0 {
        result
    } else {
        -1
    }
}

fn normalize_short(result: i32) -> i16 {
    if result >= 0 {
        result as i16
    } else {
        -1
    }
}

fn syscall0(number: usize) -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("eax") number as i32 => result,
            options(nostack),
        );
    }
    result
}

fn syscall1(number: usize, arg0: usize) -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("eax") number as i32 => result,
            in("ebx") arg0,
            options(nostack),
        );
    }
    result
}

fn syscall2(number: usize, arg0: usize, arg1: usize) -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("eax") number as i32 => result,
            in("ebx") arg0,
            in("ecx") arg1,
            options(nostack),
        );
    }
    result
}

fn syscall3(number: usize, arg0: usize, arg1: usize, arg2: usize) -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("eax") number as i32 => result,
            in("ebx") arg0,
            in("ecx") arg1,
            in("edx") arg2,
            options(nostack),
        );
    }
    result
}
