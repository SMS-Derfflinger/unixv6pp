use alloc::boxed::Box;
use kernel_macros::define_class_compat;

use crate::{
    constants::{PosixError, SIGMAX},
    fs::{DirectoryEntry, IOParameter, InodeRef, OpenFiles},
};

pub struct Pointer(usize);

pub struct Process {
    uid: u16,
}

#[repr(C)]
pub struct MemoryDescriptor {
    /// Opaque for now...
    data: [usize; 6],
}

pub struct Userspace {
    /// Save esp and ebp
    rsav: [Pointer; 2],
    /// Save esp and ebp AGAIN
    ssav: [Pointer; 2],

    proc: &'static mut Process,
    mem: MemoryDescriptor,

    ar0: &'static mut u32,

    /// Used by syscall handlers
    args: [usize; 5],

    dirp: &'static mut u8,

    /// User time elapsed
    utime: u32,
    /// Sys time elapsed
    stime: u32,

    /// Sum of all children's user time
    children_utime: u32,
    /// Sum of all children's sys time
    children_stime: u32,

    /// Pending signals
    signals: [usize; SIGMAX],

    /// Used to jump back to Trap() on signal received
    qsav: [Pointer; 2],

    /// Do we have pending signals?
    signal_pending: bool,

    /// Inode of our working directory
    cwd: InodeRef,
    /// Inode of our pwd's parent
    cwd_parent: InodeRef,

    /// Dentry of our pwd
    dentry: DirectoryEntry,

    /// Last component of pwd
    cwd_name: [u8; 28],
    /// Full path of pwd
    cwd_full: [u8; 128],

    /// Userspace error code
    error: Option<PosixError>,

    uid: u16,
    gid: u16,
    euid: u16,
    egid: u16,

    pub open_files: OpenFiles,
    ioparam: IOParameter,
}

impl Userspace {
    pub fn get() -> &'static mut Self {
        const RUST_USER_ADDRESS: usize = 0xc03ff200;
        unsafe { &mut *(RUST_USER_ADDRESS as *mut Self) }
    }

    fn is_root(&mut self) -> bool {
        if self.uid == 0 {
            return true;
        }

        self.error = Some(PosixError::EPERM);
        false
    }

    fn setuid(&mut self, uid: u16, suid: u16) {
        if self.euid == uid || self.is_root() {
            self.uid = uid;
            self.euid = uid;
            self.proc.uid = uid;
        } else {
            self.error = Some(PosixError::EPERM);
        }
    }

    fn getuid(&self) -> u32 {
        ((self.uid as u32) << 16) | ((self.euid as u32) & 0xff)
    }

    fn setgid(&mut self, gid: u16, sgid: u16) {
        if self.egid == gid || self.is_root() {
            self.gid = gid;
            self.egid = gid;
        } else {
            self.error = Some(PosixError::EPERM);
        }
    }

    fn getgid(&self) -> u32 {
        ((self.gid as u32) << 16) | ((self.egid as u32) & 0xff)
    }

    fn getpwd(&self) -> [u8; 128] {
        self.cwd_full.clone()
    }
}

macro_rules! define_user_compat {
{ $( $rust_ident:ident: $type:ty => $c_ident:ident := $init:expr; )* } => {
    struct SaveHandle {
        $(
            $rust_ident: $type,
        )*
    }

    define_class_compat!{impl Userspace {
        pub fn before_fork() -> Box<SaveHandle> {
            let user = Userspace::get();

            Box::new(SaveHandle {
                $(
                    $rust_ident: user.$rust_ident.clone(),
                )*
            })
        }

        pub fn after_fork(handle: Box<SaveHandle>) {
            let user = Userspace::get();

            $(
                user.$rust_ident = handle.$rust_ident;
            )*
        }

        pub fn init() {
            let user = Userspace::get();

            unsafe {
                $(
                    (&raw mut user.$rust_ident).write($init);
                )*
            }
        }
    }}

    define_class_compat! {impl User {
        $(
            pub fn $c_ident() -> *mut $type {
                &raw mut Userspace::get().$rust_ident
            }
        )*
    }}
};
}

define_user_compat! {
    signal_pending: bool => get_intflg_ := false;
    signals: [usize; SIGMAX] => get_signal_ := [0; SIGMAX];
    open_files: OpenFiles => get_ofiles_ := OpenFiles::new();
    ioparam: IOParameter => get_IOParam_ := IOParameter::new();
}
