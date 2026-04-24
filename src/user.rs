use core::ffi::CStr;

use alloc::boxed::Box;
use eonix_mm::paging::PFN;
use kernel_macros::define_class_compat;

use crate::{
    compat::compat_flush_page_directory,
    constants::{PosixError, Signal, SIGMAX},
    fs::{DirectoryEntry, IOParameter, InodeRef, OpenFiles},
    machine::{global_user_page_table, EntryFlags, PageTable, PageTableEntry},
    mm::PAGE_SIZE,
    proc::Process,
    serial::KResult,
};

#[derive(Clone, Copy)]
pub struct Pointer(pub usize);

pub struct Terminal;

#[repr(C)]
#[derive(Clone)]
pub struct MemoryDescriptor {
    pub user_pts: Box<[PageTable; 2]>,
    pub text: usize,
    pub text_len: usize,
    pub data: usize,
    pub data_len: usize,
    pub stack_len: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct Userspace {
    /// Save esp and ebp
    rsav: [Pointer; 2],
    /// Save esp and ebp AGAIN
    pub ssav: [Pointer; 2],

    pub proc: *mut Process,
    pub mem: MemoryDescriptor,

    pub ar0: *mut u32,

    /// Used by syscall handlers
    pub args: [usize; 5],

    pub dirp: *mut u8,

    /// User time elapsed
    pub utime: u32,
    /// Sys time elapsed
    pub stime: u32,

    /// Sum of all children's user time
    pub children_utime: u32,
    /// Sum of all children's sys time
    pub children_stime: u32,

    /// Pending signals
    signals: [usize; SIGMAX],

    /// Used to jump back to Trap() on signal received
    qsav: [Pointer; 2],

    /// Do we have pending signals?
    pub signal_pending: bool,

    /// Inode of our working directory
    pub cwd: Option<InodeRef>,
    /// Inode of our pwd's parent
    pub cwd_parent: Option<InodeRef>,
    /// Dentry of our pwd
    pub dentry: DirectoryEntry,

    /// Last component of pwd
    cwd_name: [u8; 28],
    /// Full path of pwd
    pub cwd_full: [u8; 128],

    /// Userspace error code
    pub error: Option<PosixError>,

    pub uid: u16,
    pub gid: u16,
    pub euid: u16,
    pub egid: u16,

    pub open_files: OpenFiles,
    pub ioparam: IOParameter,
}

impl Userspace {
    pub fn get() -> &'static mut Self {
        const RUST_USER_ADDRESS: usize = 0xc03ff000;
        unsafe { &mut *(RUST_USER_ADDRESS as *mut Self) }
    }

    pub fn replace(swap: &mut Box<Self>) {
        core::mem::swap(swap.as_mut(), Userspace::get());
    }

    pub fn new() -> Self {
        Self {
            signal_pending: false,
            signals: [0; SIGMAX],
            open_files: OpenFiles::new(),
            ioparam: IOParameter::new(),
            utime: 0,
            stime: 0,
            children_utime: 0,
            children_stime: 0,
            uid: 0,
            euid: 0,
            gid: 0,
            egid: 0,
            args: [0; 5],
            dirp: core::ptr::null_mut(),
            cwd_full: {
                let mut arr = [0; 128];
                arr[0] = b'/';
                arr
            },
            dentry: DirectoryEntry::new(),
            cwd_name: [0; 28],
            mem: MemoryDescriptor::new(),
            ar0: core::ptr::null_mut(),
            proc: core::ptr::null_mut(),
            cwd: None,
            cwd_parent: None,
            error: None,
            rsav: [Pointer(0); 2],
            ssav: [Pointer(0); 2],
            qsav: [Pointer(0); 2],
        }
    }

    pub fn set_error(&mut self, errno: PosixError) {
        self.error = Some(errno);
    }

    pub fn set_user_retval(&mut self, retval: u32) {
        unsafe {
            self.ar0.write(retval);
        }
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn io_param_mut(&mut self) -> &mut IOParameter {
        &mut self.ioparam
    }

    pub fn is_root(&mut self) -> bool {
        if self.uid == 0 {
            return true;
        }

        self.error = Some(PosixError::EPERM);
        false
    }

    pub fn setuid(&mut self, uid: u16, _suid: u16) {
        if self.euid == uid || self.is_root() {
            self.uid = uid;
            self.euid = uid;
            self.proc().setuid(uid);
        } else {
            self.error = Some(PosixError::EPERM);
        }
    }

    pub fn getuid(&self) -> u32 {
        ((self.uid as u32) << 16) | ((self.euid as u32) & 0xff)
    }

    pub fn setgid(&mut self, gid: u16, _sgid: u16) {
        if self.egid == gid || self.is_root() {
            self.gid = gid;
            self.egid = gid;
        } else {
            self.error = Some(PosixError::EPERM);
        }
    }

    pub fn getgid(&self) -> u32 {
        ((self.gid as u32) << 16) | ((self.egid as u32) & 0xff)
    }

    fn getpwd(&self) -> [u8; 128] {
        self.cwd_full.clone()
    }

    pub fn argdir(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.dirp as *const i8).to_bytes() }
    }

    pub fn argdir_mut(&mut self) -> &mut [u8; 28] {
        &mut self.cwd_name
    }

    pub fn getcwd(&self) -> InodeRef {
        self.cwd
            .clone()
            .expect("current working directory is not set")
    }

    pub fn set_cwd_parent(&mut self, parent: InodeRef) {
        self.cwd_parent = Some(parent);
    }

    pub fn proc(&self) -> &'static mut Process {
        unsafe { &mut *self.proc }
    }

    pub fn set_signal_handler(&mut self, signal: Signal, func: usize) {
        self.signals[signal as usize] = func;
    }

    pub fn clear_signal_handlers(&mut self) {
        for signal in &mut self.signals {
            *signal = 0;
        }
    }

    pub fn get_signal_handler(&self, signal: Signal) -> usize {
        self.signals[signal as usize]
    }
}

impl Drop for MemoryDescriptor {
    fn drop(&mut self) {
        crate::println_debug!("drop: {:p}", &self.user_pts);
    }
}

impl MemoryDescriptor {
    pub const USER_SPACE_SIZE: usize = 0x800000;
    pub const USER_SPACE_START: usize = 0;
    pub const USER_SPACE_END: usize = Self::USER_SPACE_START + Self::USER_SPACE_SIZE;

    pub fn new() -> Self {
        let user_pts = unsafe { Box::new_zeroed().assume_init() };
        crate::println_debug!("alloc: {:p}", &user_pts);

        Self {
            user_pts,
            text: 0,
            text_len: 0,
            data: 0,
            data_len: 0,
            stack_len: 0,
        }
    }

    pub fn len(&self) -> usize {
        PAGE_SIZE + self.text_len + self.data_len + self.stack_len
    }

    pub fn end(&self) -> usize {
        self.text + self.len()
    }

    pub fn overflow(&self) -> bool {
        self.end() > Self::USER_SPACE_SIZE
    }

    fn user_ptes(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.user_pts.iter_mut().map(|tbl| tbl.iter_mut()).flatten()
    }

    fn clear_user(&mut self) {
        for pte in self.user_ptes() {
            pte.set(None, EntryFlags::USER);
        }
    }

    /// Map len bytes from physical address pfn -> virtual address addr
    ///
    /// # Returns
    /// The pfn following the last mapped page.
    fn map_range(&mut self, addr: usize, len: usize, mut pfn: PFN, rw: bool) -> PFN {
        let addr = addr - Self::USER_SPACE_START;
        let pt_idx = addr >> 12;
        let cnt = (len + PAGE_SIZE - 1) / PAGE_SIZE;

        let mut flags = EntryFlags::PRESENT | EntryFlags::USER;
        if rw {
            flags |= EntryFlags::WRITE;
        }

        for pte in self.user_ptes().skip(pt_idx).take(cnt) {
            pte.set(Some(pfn), flags);
            pfn = pfn + 1;
        }
        pfn
    }

    pub fn map_to_actual_pt(&mut self, proc: &Process) {
        let Some(text) = &proc.text else { return };
        let text_pfn = text.pfn().unwrap();
        let data_pfn = proc.addr >> 12;

        self.do_map_to_actual_pt(usize::from(text_pfn), data_pfn)
    }

    fn do_map_to_actual_pt(&mut self, text_pfn: usize, data_pfn: usize) {
        let real_pt = global_user_page_table();

        let ptes = real_pt.iter_mut().map(|pt| pt.iter_mut()).flatten();

        for (pte, fake_pte) in ptes.zip(self.user_ptes()) {
            let (mut pfn, flags) = fake_pte.get();

            if !flags.contains(EntryFlags::PRESENT) {
                pte.set(None, EntryFlags::empty());
                continue;
            }

            if flags.contains(EntryFlags::WRITE) {
                pfn = pfn + data_pfn;
            } else {
                pfn = pfn + text_pfn;
            }

            pte.set(Some(pfn), flags);
        }

        real_pt[0][0].set(
            Some(PFN::from_val(0)),
            EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::USER,
        );

        compat_flush_page_directory();
    }

    pub fn establish_user(&mut self, proc: &Process) -> KResult<()> {
        let text = proc.text.as_ref().unwrap();
        let text_pfn = text.pfn().unwrap();
        let data_pfn = proc.addr >> 12;

        self.do_establish_user(usize::from(text_pfn), data_pfn)
    }

    fn do_establish_user(&mut self, text_pfn: usize, data_pfn: usize) -> KResult<()> {
        if self.overflow() {
            crate::println_warn!("Process address space overflow");
            return Err(PosixError::ENOMEM);
        }

        self.clear_user();

        self.map_range(self.text, self.text_len, PFN::from_val(0), false);
        let data_end = self.map_range(self.data, self.data_len, PFN::from_val(1), true);
        self.map_range(
            Self::USER_SPACE_END - self.stack_len,
            self.stack_len,
            data_end,
            true,
        );
        self.do_map_to_actual_pt(text_pfn, data_pfn);

        Ok(())
    }
}

macro_rules! define_user_compat {
{ $( $rust_ident:ident: $type:ty => $c_ident:ident := $init:expr; )* } => {
    define_class_compat!{impl Userspace {
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
    utime: u32 => get_utime_ := 0;
    stime: u32 => get_stime_ := 0;
    children_utime: u32 => get_cutime_ := 0;
    children_stime: u32 => get_cstime_ := 0;
    uid: u16 => get_uid_ := 0;
    euid: u16 => get_ruid_ := 0;
    gid: u16 => get_gid_ := 0;
    egid: u16 => get_rgid_ := 0;
    args: [usize; 5] => get_arg_ := [0; 5];
    dirp: *mut u8 => get_dirp_ := core::ptr::null_mut();
    cwd_full: [u8; 128] => get_curdir_ := {
        let mut arr = [0; 128]; arr[0] = b'/'; arr
    };
    dentry: DirectoryEntry => get_dent_ := DirectoryEntry::new();
    cwd_name: [u8; 28] => get_dbuf_ := [0; 28];
    mem: MemoryDescriptor => get_MemoryDescriptor_ := MemoryDescriptor::new();
    ar0: *mut u32 => get_ar0_ := core::ptr::null_mut();
    proc: *mut Process => get_procp_ := core::ptr::null_mut();
    cwd: Option<InodeRef> => get_cdir_ := None;
    cwd_parent: Option<InodeRef> => get_pdir_ := None;
    error: Option<PosixError> => get_error_ := None;
    rsav: [Pointer; 2] => get_rsav_ := [Pointer(0); 2];
    ssav: [Pointer; 2] => get_ssav_ := [Pointer(0); 2];
    qsav: [Pointer; 2] => get_qsav_ := [Pointer(0); 2];
}

define_class_compat! {impl Userspace {
    pub fn is_root(&mut self) -> bool {
        this.is_root()
    }
}}
