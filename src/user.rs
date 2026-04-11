use crate::{
    constants::{PosixError, SIGMAX},
    fs::{IOParameter, Inode, OpenFiles},
};

pub struct Pointer(usize);

pub struct Process {
    uid: u16,
}

pub struct MemoryDescriptor {}

pub struct Dentry;

pub struct Userspace {
    /// Save esp and ebp
    rsav: [Pointer; 2],
    /// Save esp and ebp AGAIN
    ssav: [Pointer; 2],

    proc: &'static mut Process,
    mem: MemoryDescriptor,

    /// Used by syscall handlers
    args: [usize; 5],

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
    /// Do we have pending signals?
    signal_pending: bool,

    /// Used to jump back to Trap() on signal received
    qsav: [Pointer; 2],

    /// Inode of our working directory
    cwd: &'static Inode,
    /// Inode of our pwd's parent
    cwd_parent: &'static Inode,

    /// Dentry of our pwd
    dentry: &'static Dentry,

    /// Last component of pwd
    cwd_name: [u8; 28],
    /// Full path of pwd
    cwd_full: [u8; 28],

    /// Userspace error code
    error: Option<PosixError>,
    /// Is I/O in user space or kernel space
    seg_flag: u32,

    uid: u16,
    gid: u16,
    euid: u16,
    egid: u16,

    open_files: OpenFiles,
    ioparam: IOParameter,
}

impl Userspace {
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

    fn getpwd(&self) -> [u8; 28] {
        self.cwd_full.clone()
    }
}
