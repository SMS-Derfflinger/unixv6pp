use core::array;

use crate::{
    constants::PosixError,
    fs::{
        self,
        file::{File, FileFlags, OpenFiles},
        inode::{DevId, INodeFlag, INodeMode, Inode, PhysicalBlock},
    },
};

pub(crate) struct OpenFileTable {
    m_file: [File; Self::NFILE],
}

impl OpenFileTable {
    const NFILE: usize = 100;

    pub const fn new_const() -> Self {
        Self {
            m_file: [const { File::new_const() }; Self::NFILE],
        }
    }

    pub fn new() -> Self {
        Self {
            m_file: array::from_fn(|_| File::new()),
        }
    }

    /// 在系统打开文件表中分配一个空闲 File，
    /// 同时在进程打开文件描述符表中分配对应 fd。
    /// 返回 (fd, &mut File) 或 Err。
    /// TODO: user args and consider Rc and mutex
    pub fn f_alloc<'a>(
        &'a mut self,
        open_files: &mut OpenFiles,
    ) -> Result<(usize, &'a mut File), PosixError> {
        let fd = open_files.alloc_free_slot()?;

        for file in self.m_file.iter_mut() {
            if file.f_count == 0 {
                file.f_count = 1;
                file.f_offset = 0;
                file.f_flag = FileFlags::empty();
                file.f_inode = None;
                open_files.set_f(fd, file as *mut File);
                return Ok((fd, file));
            }
        }

        Err(PosixError::ENFILE)
    }

    pub fn close_f(&mut self, file: &mut File) {
        let inode_table: &mut InodeTable = unsafe { &mut fs::global_inode_table() };
        if file.f_flag.contains(FileFlags::FPIPE) {
            if let Some(idx) = file.f_inode {
                let inode = &mut inode_table.m_inode[idx];
                inode.i_mode &= !(INodeMode::IREAD | INodeMode::IWRITE);
                // TODO: wake up
                // proc_mgr.wake_up_all(idx + 1);
                // proc_mgr.wake_up_all(idx + 2);
            }
        }

        if file.f_count <= 1 {
            if let Some(idx) = file.f_inode {
                let write_flag = if file.f_flag.contains(FileFlags::FWRITE) {
                    1
                } else {
                    0
                };
                inode_table.m_inode[idx].close_i(write_flag);
                inode_table.i_put(idx);
            }
        }

        file.f_count -= 1;
    }
}

pub(crate) struct InodeTable {
    pub m_inode: [Inode; InodeTable::NINODE],
}

impl InodeTable {
    pub const NINODE: usize = 100;

    pub const fn new_const() -> Self {
        Self {
            m_inode: [const { Inode::new_const() }; Self::NINODE],
        }
    }

    pub fn new() -> Self {
        Self {
            m_inode: array::from_fn(|_| Inode::new()),
        }
    }

    pub fn i_get(&mut self, mut dev: DevId, mut inumber: i32) -> Result<usize, PosixError> {
        loop {
            if let Some(idx) = self.is_loaded(dev, inumber) {
                let inode = &mut self.m_inode[idx];

                if inode.i_flag.contains(INodeFlag::ILOCK) {
                    // TODO: sleep
                    inode.i_flag |= INodeFlag::IWANT;
                    //kernel::get_process_manager()
                    //    .sleep(inode as *mut _ as usize, ProcessManager::PINOD);
                }

                if inode.i_flag.contains(INodeFlag::IMOUNT) {
                    // TODO: fs
                    //let mount = kernel::get_filesystem()
                    //    .get_mount(idx)
                    //    .ok_or(Error::PANIC)?;
                    //dev     = mount.m_dev;
                    //inumber = FileSystem::ROOTINO;
                    continue;
                }

                inode.i_count += 1;
                inode.i_flag |= INodeFlag::ILOCK;
                return Ok(idx);
            }

            let idx = self.get_free_inode().ok_or(PosixError::ENFILE)?;
            {
                let inode = &mut self.m_inode[idx];
                inode.i_dev = dev;
                inode.i_number = inumber;
                inode.i_flag = INodeFlag::ILOCK;
                inode.i_count = 1;
                inode.i_lastr = -1;
            }

            // TODO: fs
            //let sector = FileSystem::INODE_ZONE_START_SECTOR
            //    + inumber as usize / FileSystem::INODE_NUMBER_PER_SECTOR;

            // TODO: buffer
            //let buf = kernel::buf_bread(dev, PhysicalBlock(sector as u32));

            //if buf.b_flags.contains(BufFlag::B_ERROR) {
            //    drop(buf);
            //    self.i_put(idx);
            //    return Err(Error::EIO);
            //}

            //self.m_inode[idx].i_copy(&buf, inumber as usize);
            return Ok(idx);
        }
    }

    /// 减少引用计数，计数归零时将 inode 写回磁盘并释放。
    pub fn i_put(&mut self, idx: usize) {
        let inode = &mut self.m_inode[idx];

        if inode.i_count == 1 {
            inode.i_flag |= INodeFlag::ILOCK;

            if inode.i_nlink <= 0 {
                inode.i_trunc();
                inode.i_mode = INodeMode::empty();
                // TODO: fs
                //kernel::get_filesystem().i_free(inode.i_dev, inode.i_number);
            }

            // TODO: time
            inode.i_update(0);
            inode.prele();
            inode.i_flag = INodeFlag::empty();
            inode.i_number = -1;
        }

        inode.i_count -= 1;
        inode.prele();
    }

    pub fn update_inode_table(&mut self) {
        for inode in self.m_inode.iter_mut() {
            if !inode.i_flag.contains(INodeFlag::ILOCK) && inode.i_count != 0 {
                inode.i_flag |= INodeFlag::ILOCK;
                // TODO: time
                inode.i_update(0);
                inode.prele();
            }
        }
    }

    pub fn is_loaded(&self, dev: DevId, inumber: i32) -> Option<usize> {
        self.m_inode
            .iter()
            .position(|inode| inode.i_dev == dev && inode.i_number == inumber && inode.i_count != 0)
    }

    pub fn get_free_inode(&self) -> Option<usize> {
        self.m_inode.iter().position(|inode| inode.i_count == 0)
    }
}
