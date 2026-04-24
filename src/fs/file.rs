use alloc::sync::Arc;
use bitflags::bitflags;
use core::array;
use eonix_spin::Spin;

use crate::{
    constants::PosixError,
    fs::{FileRef, InodeRef},
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FileFlags: u32 {
        const FREAD  = 0x1;  // 读请求
        const FWRITE = 0x2;  // 写请求
        const FPIPE  = 0x4;  // 管道
    }
}

#[repr(C)]
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

    pub fn new() -> Arc<Spin<File>> {
        Arc::new(Spin::new(Self::new_const()))
    }

    pub fn is_free(&self) -> bool {
        self.f_count == 0
    }

    pub fn reset_for_open(&mut self) {
        self.f_count = 1;
        self.f_offset = 0;
        self.f_flag = FileFlags::empty();
        self.f_inode = None;
    }
}

#[derive(Clone)]
pub struct OpenFiles {
    table: [Option<FileRef>; 15],
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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
