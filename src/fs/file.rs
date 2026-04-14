use alloc::{boxed::Box, rc::Rc, sync::Arc};
use bitflags::bitflags;
use core::{array, cell::RefCell};
use eonix_spin::Spin;

use crate::{
    constants::PosixError,
    define_class_compat,
    fs::{FileRef, InodeRef},
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

pub struct File {
    pub f_flag: FileFlags,
    pub f_count: i32,
    pub f_inode: Option<InodeRef>,
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

#[no_mangle]
pub extern "C" fn ofiles_alloc_free_slot(
    ofiles: &mut OpenFiles,
    err: &mut Option<PosixError>,
) -> i32 {
    match ofiles.alloc_free_slot() {
        Ok(fd) => {
            *err = None;
            fd as i32
        }
        Err(e) => {
            *err = Some(e);
            -1
        }
    }
}

define_class_compat! {
    impl OpenFiles {
        pub fn ofiles_get_file(fd: i32, err: &mut Option<PosixError>) -> *const Spin<File> {
            if fd < 0 {
                *err = Some(PosixError::EBADF);
                return core::ptr::null();
            }

            match ofiles.get_f(fd as usize) {
                Ok(file) => {
                    *err = None;
                    Arc::into_raw(file)
                }
                Err(e) => {
                    *err = Some(e);
                    core::ptr::null()
                }
            }
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
