use bitflags::bitflags;

use crate::constants::PosixError;

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
    pub f_inode: Option<usize>, // InodeTable 中的下标
    pub f_offset: i32,
}

impl File {
    pub const fn new_const() -> Self {
        Self {
            f_flag: FileFlags::empty(),
            f_count: 0,
            f_inode: None,
            f_offset: 0,
        }
    }

    pub fn new() -> Self {
        Self {
            f_flag: FileFlags::empty(),
            f_count: 0,
            f_inode: None,
            f_offset: 0,
        }
    }
}

pub struct OpenFiles {
    table: [Option<*mut File>; OpenFiles::NOFILES],
}

impl OpenFiles {
    pub const NOFILES: usize = 15;

    pub fn new() -> Self {
        Self {
            table: [None; OpenFiles::NOFILES],
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

    pub fn clone_fd(&self, fd: usize) -> Result<usize, PosixError> {
        // TODO
        Ok(0)
    }

    pub fn get_f(&self, fd: usize) -> Result<*mut File, PosixError> {
        if fd >= Self::NOFILES {
            return Err(PosixError::EBADF);
        }
        self.table[fd].ok_or(PosixError::EBADF)
    }

    pub fn set_f(&mut self, fd: usize, file: *mut File) {
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
