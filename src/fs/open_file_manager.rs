use alloc::rc::Rc;
use core::array;
use core::cell::RefCell;

use crate::{
    constants::PosixError,
    fs::{
        self,
        file::{File, FileFlags, FileRef, InodeRef, OpenFiles},
        file_system::FileSystem,
        inode::{DevId, INodeFlag, INodeMode, Inode},
    },
};

pub(crate) struct OpenFileTable {
    m_file: [FileRef; Self::NFILE],
}

// SAFETY: migration-stage design. Table mutations are serialized by the outer global Spin lock.
unsafe impl Send for OpenFileTable {}

impl OpenFileTable {
    const NFILE: usize = 100;

    pub fn new() -> Self {
        Self {
            m_file: array::from_fn(|_| Rc::new(RefCell::new(File::new()))),
        }
    }

    /// 在系统打开文件表中分配一个空闲 File，
    /// 同时在进程打开文件描述符表中分配对应 fd。
    /// 返回 (fd, Rc<RefCell<File>>) 或 Err。
    pub fn f_alloc(&mut self, open_files: &mut OpenFiles) -> Result<(usize, FileRef), PosixError> {
        let fd = open_files.alloc_free_slot()?;

        for file in &self.m_file {
            if file.borrow().f_count == 0 {
                {
                    let mut file = file.borrow_mut();
                    file.f_count = 1;
                    file.f_offset = 0;
                    file.f_flag = FileFlags::empty();
                    file.f_inode = None;
                }
                open_files.set_f(fd, Rc::clone(file));
                return Ok((fd, Rc::clone(file)));
            }
        }

        Err(PosixError::ENFILE)
    }

    pub fn close_f(&mut self, file: &FileRef) {
        let mut file = file.borrow_mut();

        if file.f_flag.contains(FileFlags::FPIPE) {
            if let Some(inode) = file.f_inode.as_ref() {
                let mut inode = inode.borrow_mut();
                inode.i_mode &= !(INodeMode::IREAD | INodeMode::IWRITE);
                // TODO: wake up
                // proc_mgr.wake_up_all((&*inode as *const Inode as usize) + 1);
                // proc_mgr.wake_up_all((&*inode as *const Inode as usize) + 2);
            }
        }

        if file.f_count <= 1 {
            if let Some(inode) = file.f_inode.take() {
                let write_flag = if file.f_flag.contains(FileFlags::FWRITE) {
                    1
                } else {
                    0
                };
                inode.borrow().close_i(write_flag);
                fs::global_inode_table().i_put(inode);
            }
        }

        file.f_count -= 1;
    }
}

pub(crate) struct InodeTable {
    pub m_inode: [InodeRef; InodeTable::NINODE],
}

// SAFETY: migration-stage design. Table mutations are serialized by the outer global Spin lock.
unsafe impl Send for InodeTable {}

impl InodeTable {
    pub const NINODE: usize = 100;

    pub fn new() -> Self {
        Self {
            m_inode: array::from_fn(|_| Rc::new(RefCell::new(Inode::new()))),
        }
    }

    pub fn i_get(&mut self, dev: DevId, inumber: i32) -> Result<InodeRef, PosixError> {
        let mut dev = dev;
        let mut inumber = inumber;

        loop {
            if let Some(inode) = self.is_loaded(dev, inumber) {
                let mut inode_ref = inode.borrow_mut();

                if inode_ref.i_flag.contains(INodeFlag::ILOCK) {
                    // TODO: sleep
                    inode_ref.i_flag |= INodeFlag::IWANT;
                    //kernel::get_process_manager()
                    //    .sleep(inode as *mut _ as usize, ProcessManager::PINOD);
                }

                if inode_ref.i_flag.contains(INodeFlag::IMOUNT) {
                    let mount_dev = fs::global_file_system()
                        .get_mount(&inode_ref)
                        .map(|mount| mount.m_dev);
                    drop(inode_ref);

                    if let Some(mount_dev) = mount_dev {
                        dev = mount_dev;
                        inumber = FileSystem::ROOTINO;
                        continue;
                    }

                    return Err(PosixError::ENOENT);
                }

                inode_ref.i_count += 1;
                inode_ref.i_flag |= INodeFlag::ILOCK;
                drop(inode_ref);
                return Ok(inode);
            }

            let inode = self.get_free_inode().ok_or(PosixError::ENFILE)?;
            {
                let mut inode = inode.borrow_mut();
                inode.i_dev = dev;
                inode.i_number = inumber;
                inode.i_flag = INodeFlag::ILOCK;
                inode.i_count = 1;
                inode.i_lastr = -1;
            }

            let _sector = FileSystem::INODE_ZONE_START_SECTOR
                + inumber as usize / FileSystem::INODE_NUMBER_PER_SECTOR;

            // TODO: buffer
            //let buf = kernel::buf_bread(dev, PhysicalBlock(sector as u32));

            //if buf.b_flags.contains(BufFlag::B_ERROR) {
            //    drop(buf);
            //    self.i_put(inode.clone());
            //    return Err(Error::EIO);
            //}

            //inode.borrow_mut().i_copy(&buf, inumber as usize);
            return Ok(inode);
        }
    }

    /// 减少引用计数，计数归零时将 inode 写回磁盘并释放。
    pub fn i_put(&mut self, inode: InodeRef) {
        let mut inode = inode.borrow_mut();

        if inode.i_count == 1 {
            inode.i_flag |= INodeFlag::ILOCK;

            if inode.i_nlink <= 0 {
                inode.i_trunc();
                inode.i_mode = INodeMode::empty();
                let _ = fs::global_file_system().i_free(inode.i_dev, inode.i_number);
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
        for inode in &self.m_inode {
            let mut inode = inode.borrow_mut();
            if !inode.i_flag.contains(INodeFlag::ILOCK) && inode.i_count != 0 {
                inode.i_flag |= INodeFlag::ILOCK;
                // TODO: time
                inode.i_update(0);
                inode.prele();
            }
        }
    }

    pub fn is_loaded(&self, dev: DevId, inumber: i32) -> Option<InodeRef> {
        self.m_inode
            .iter()
            .find(|inode| {
                let inode = inode.borrow();
                inode.i_dev == dev && inode.i_number == inumber && inode.i_count != 0
            })
            .cloned()
    }

    pub fn get_free_inode(&self) -> Option<InodeRef> {
        self.m_inode
            .iter()
            .find(|inode| inode.borrow().i_count == 0)
            .cloned()
    }
}
