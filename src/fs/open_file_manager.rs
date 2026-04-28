use eonix_spin::Spin;
use eonix_sync_base::LazyLock;

use crate::{
    constants::{fs_constants, PosixError},
    dev::{
        buffer::{Buffer, DevId, PhysicalBlock},
        buffer_manager::global_buffer_manager,
    },
    fs::{
        self,
        file::{File, FileFlags, OpenFiles},
        file_system::FileSystem,
        inode::{DiskInode, Inode, InodeFlag, InodeMode},
        FileRef, FileSlot, InodeRef, InodeRefGuard, InodeSlot,
    },
    interrupt::time::get_time,
    proc::{Channel, ProcessManager, PINOD},
    sync::{IrqGuard, Mutex, SpinExt},
    user::Userspace,
    Ext,
};

pub(crate) struct OpenFileTable {
    m_file: [FileSlot; Self::NFILE],
}

impl OpenFileTable {
    const NFILE: usize = 100;

    pub fn new() -> Self {
        Self {
            m_file: core::array::from_fn(|_| FileSlot(File::new())),
        }
    }

    fn find_free(&mut self) -> Option<&mut FileSlot> {
        for file in &mut self.m_file {
            if !file.0.lock().is_free() {
                continue;
            }

            return Some(file);
        }

        None
    }

    /// 在系统打开文件表中分配一个空闲 File，
    /// 同时在进程打开文件描述符表中分配对应 fd。
    /// 返回 (fd, FileRef) 或 Err。
    pub fn f_alloc(&mut self, open_files: &mut OpenFiles) -> Result<(usize, FileRef), PosixError> {
        let fd = open_files.alloc_free_slot()?;
        let free_file = self.find_free().ok_or(PosixError::ENFILE)?;

        {
            let mut file = free_file.0.lock();
            file.reset_for_open();
            file.f_count += 1;
        }
        let fd_ref = FileRef::from_slot_owned(free_file.clone());
        let ret_ref = FileRef::from_slot_owned(free_file.clone());
        open_files.set_f(fd, fd_ref);
        Ok((fd, ret_ref))
    }

    fn close_pipe(&mut self, file: &mut File) {
        let inoderef = file.f_inode.as_ref().expect("Pipe without inodes");
        let mut inode = inoderef.lock();

        inode.i_mode &= !(InodeMode::IREAD | InodeMode::IWRITE);
        ProcessManager::get().wakeup_all(inode.channel_read());
        ProcessManager::get().wakeup_all(inode.channel_write());
    }

    pub fn f_dup(&mut self, slot: &FileSlot) {
        slot.0.lock().f_count += 1;
    }

    pub fn f_put_slot(&mut self, slot: &FileSlot) {
        let mut file = slot.0.lock();

        if file.f_flag.contains(FileFlags::FPIPE) {
            self.close_pipe(&mut file);
        }

        if file.f_count <= 1 {
            if let Some(inode) = file.f_inode.take() {
                inode.lock().close_i(file.f_flag & FileFlags::FWRITE);
            }
        }

        file.f_count -= 1;
    }
}

pub(crate) struct InodeTable {
    pub m_inode: [InodeSlot; InodeTable::NINODE],
}

impl InodeTable {
    pub const NINODE: usize = 100;

    pub fn new() -> Self {
        Self {
            m_inode: core::array::from_fn(|_| InodeSlot(Inode::new())),
        }
    }

    fn ino_blkoff(ino: i32) -> usize {
        (ino as usize % fs_constants::INODE_NUMBER_PER_SECTOR) * size_of::<DiskInode>()
    }

    fn icopy(inode: &mut Inode, buf: Buffer, ino: i32) {
        let mut disk_inode = DiskInode::default();
        let data = &buf.as_bytes()[Self::ino_blkoff(ino)..];

        disk_inode
            .as_buffer()
            .copy_from_slice(&data[..size_of::<DiskInode>()]);

        inode.i_mode = disk_inode.d_mode;
        inode.i_nlink = disk_inode.d_nlink;
        inode.i_uid = disk_inode.d_uid;
        inode.i_gid = disk_inode.d_gid;
        inode.i_size = disk_inode.d_size;
        inode.i_addr = disk_inode.d_addr;
    }

    pub fn i_get(&mut self, dev: DevId, ino: i32) -> Result<InodeRefGuard, PosixError> {
        loop {
            let Some(iref) = self.get(dev, ino) else {
                let iref = self.alloc_free(dev, ino).ok_or(PosixError::ENFILE)?;
                let sector = fs_constants::INODE_SECTOR_OFF as u32
                    + ino as u32 / fs_constants::INODE_NUMBER_PER_SECTOR as u32;

                let buf = global_buffer_manager().bread(dev, PhysicalBlock(sector))?;
                Self::icopy(&mut *iref.0.lock(), buf, ino);
                return Ok(InodeRefGuard::new(InodeRef::from_slot_owned(iref)));
            };

            let mut inode = iref.0.lock();

            if inode.i_flag.contains(InodeFlag::ILOCK) {
                inode.i_flag.insert(InodeFlag::IWANT);
                let chan = (&*inode).channel_addr();
                let ctx = IrqGuard::disable_save();
                drop(inode);
                Userspace::get()
                    .proc()
                    .sleep_kernel_with_irq_guard(chan, PINOD, ctx);
                continue;
            }

            if inode.i_flag.contains(InodeFlag::IMOUNT) {
                todo!("Submounts");
                continue;
                // let mount_dev = fs::global_file_system()
                //     .get_mount(&inode_ref)
                //     .map(|mount| mount.m_dev);
                // drop(inode_ref);
                // if let Some(mount_dev) = mount_dev {
                //     dev = mount_dev;
                //     ino = FileSystem::ROOTINO;
                //     continue;
                // }
                // return Err(PosixError::ENOENT);
            }

            inode.i_count += 1;
            inode.i_flag.insert(InodeFlag::ILOCK);

            drop(inode);
            return Ok(InodeRefGuard::new(InodeRef::from_slot_owned(iref)));
        }
    }

    pub fn i_dup(&mut self, slot: &InodeSlot) {
        slot.0.lock().i_count += 1;
    }

    /// 减少引用计数，计数归零时将 inode 写回磁盘并释放。
    pub fn i_put_slot(&mut self, slot: &InodeSlot) {
        let mut inode = slot.0.lock();

        inode.i_count -= 1;
        if inode.i_count != 0 {
            inode.prele();
            return;
        }

        // WTF?
        inode.i_flag |= InodeFlag::ILOCK;

        if inode.i_nlink <= 0 {
            inode.release();
            inode.i_mode = InodeMode::empty();
            let _ = fs::global_file_system().i_free(inode.i_dev, inode.i_number);
        }

        inode.i_update(get_time() as u32);
        inode.prele();
        inode.i_flag = InodeFlag::empty();
        inode.i_number = -1;
    }

    pub fn update_inode_table(&mut self) {
        for inode in &self.m_inode {
            let mut inode = inode.0.lock();
            if inode.i_flag.contains(InodeFlag::ILOCK) || inode.i_count == 0 {
                continue;
            }

            inode.i_flag |= InodeFlag::ILOCK;
            inode.i_update(get_time() as u32);
            inode.prele();
        }
    }

    pub fn get(&self, dev: DevId, ino: i32) -> Option<InodeSlot> {
        for iref in &self.m_inode {
            let inode = iref.0.lock();
            if inode.i_dev != dev || inode.i_number != ino || inode.i_count <= 0 {
                continue;
            }

            return Some(iref.clone());
        }

        None
    }

    pub fn alloc_free(&mut self, dev: DevId, ino: i32) -> Option<InodeSlot> {
        for iref in &self.m_inode {
            let mut inode = iref.0.lock();
            if inode.i_count != 0 {
                continue;
            }

            inode.i_dev = dev;
            inode.i_number = ino;
            inode.i_flag = InodeFlag::ILOCK;
            inode.i_count = 1;
            inode.i_lastr = -1;
            return Some(iref.clone());
        }

        None
    }
}

pub static GLOBAL_INODE_TABLE: LazyLock<Mutex<InodeTable>> =
    LazyLock::new(|| Mutex::new(InodeTable::new(), PINOD as i16));
