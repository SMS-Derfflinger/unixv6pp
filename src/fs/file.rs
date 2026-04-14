use alloc::{boxed::Box, sync::Arc};
use bitflags::bitflags;
use core::{array, ops::Deref};
use eonix_spin::Spin;
use kernel_macros::define_class_compat;

use crate::{
    constants::PosixError,
    fs::{inode::fileref_leak, open_file_manager::seterr, FileRef, InodeRef},
    sync::SpinExt,
};

use super::inode::Inode;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FileFlags: u32 {
        const FREAD  = 0x1;  // 读请求
        const FWRITE = 0x2;  // 写请求
        const FPIPE  = 0x4;  // 管道
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct InodeRefCompat(*const Inode);

unsafe impl Send for InodeRefCompat {}
unsafe impl Sync for InodeRefCompat {}

impl InodeRefCompat {
    /// Create a reference to Inode for compatibility use.
    ///
    /// # Safety
    /// The created InodeRefCompat holds **NO REFCOUNTS**. The caller MUST
    /// manage the lifetime manually.
    pub unsafe fn new(inode: &Spin<Inode>) -> Self {
        let inode = inode.lock();
        let inode_ref = &*inode;

        Self(inode_ref as *const Inode)
    }

    pub fn own(&self) -> InodeRef {
        let arc = unsafe { Arc::from_raw((&**self) as *const Spin<Inode>) };
        let ret = arc.clone();
        core::mem::forget(arc);

        ret
    }
}

impl Deref for InodeRefCompat {
    type Target = Spin<Inode>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY: InodeRefCompat invariant guarantees this.
            &*Spin::ref_from_inner(self as *const _ as *mut _)
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct FileRefCompat(*const File);

unsafe impl Send for FileRefCompat {}
unsafe impl Sync for FileRefCompat {}

impl FileRefCompat {
    /// Create a reference to File for compatibility use.
    ///
    /// # Safety
    /// The created FileRefCompat holds **NO REFCOUNTS**. The caller MUST
    /// manage the lifetime manually.
    pub unsafe fn new(file: &Spin<File>) -> Self {
        let file = file.lock();
        let file_ref = &*file;

        Self(file_ref as *const File)
    }

    pub fn own(&self) -> FileRef {
        let arc = unsafe { Arc::from_raw((&**self) as *const Spin<File>) };
        let ret = arc.clone();
        core::mem::forget(arc);

        ret
    }
}

impl Deref for FileRefCompat {
    type Target = Spin<File>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY: InodeRefCompat invariant guarantees this.
            &*Spin::ref_from_inner(self as *const _ as *mut _)
        }
    }
}

#[repr(C)]
pub struct File {
    pub f_flag: FileFlags,
    pub f_count: i32,
    pub f_inode: Option<InodeRefCompat>,
    pub f_offset: i32,
}

impl File {
    const fn new_const() -> Self {
        Self {
            f_flag: FileFlags::empty(),
            f_count: 0,
            f_inode: None,
            f_offset: 0,
        }
    }

    pub fn new() -> FileRef {
        Arc::new(Spin::new(Self::new_const()))
    }
}

pub struct OpenFiles {
    table: [Option<FileRef>; OpenFiles::NOFILES],
}

impl OpenFiles {
    pub const NOFILES: usize = 15;

    pub fn new() -> Self {
        Self {
            table: array::from_fn(|_| None),
        }
    }

    pub fn alloc_free_slot(&mut self) -> Result<usize, PosixError> {
        for (i, slot) in self.table.iter().enumerate() {
            if slot.is_none() {
                return Ok(i);
            }
        }
        Err(PosixError::EMFILE)
    }

    pub fn clone_fd(&mut self, fd: usize) -> Result<usize, PosixError> {
        let file = self.get_f(fd)?;
        let new_fd = self.alloc_free_slot()?;
        file.lock().f_count += 1;
        self.table[new_fd] = Some(file);
        Ok(new_fd)
    }

    pub fn get_f(&self, fd: usize) -> Result<FileRef, PosixError> {
        if fd >= Self::NOFILES {
            return Err(PosixError::EBADF);
        }

        self.table[fd].clone().ok_or(PosixError::EBADF)
    }

    pub fn set_f(&mut self, fd: usize, file: FileRef) {
        if fd < Self::NOFILES {
            self.table[fd] = Some(file);
        }
    }

    pub fn clear_f(&mut self, fd: usize) {
        if fd < Self::NOFILES {
            self.table[fd] = None;
        }
    }
}

#[no_mangle]
pub extern "C" fn new_open_files() -> *mut OpenFiles {
    let ofiles = Box::new(OpenFiles::new());

    Box::into_raw(ofiles)
}

#[no_mangle]
pub extern "C" fn free_open_files(ofiles: *mut OpenFiles) {
    unsafe {
        Box::from_raw(ofiles);
    }
}

define_class_compat! {
    impl OpenFiles {
        pub fn alloc_free_slot(&mut self) -> i32 {
            match this.alloc_free_slot() {
                Ok(fd) => fd as i32,
                Err(err) => {
                    seterr(err);
                    -1
                }
            }
        }

        pub fn get_file(&self, fd: i32) -> Option<FileRefCompat> {
            if fd < 0 {
                seterr(PosixError::EBADF);
                return None;
            }

            this.get_f(fd as usize)
                .inspect_err(|&err| seterr(err)).ok().map(fileref_leak)
        }

        pub fn set_file(&mut self, fd: i32, file: FileRefCompat) {
            if fd < 0 {
                return;
            }

            this.set_f(fd as _, file.own());
        }
    }
}

pub struct IOParameter {
    pub m_base: usize,   // 用户目标区首地址
    pub m_offset: usize, // 文件字节偏移量
    pub m_count: usize,  // 剩余读写字节数
}

impl IOParameter {
    pub fn new() -> Self {
        Self {
            m_base: 0,
            m_offset: 0,
            m_count: 0,
        }
    }
}
