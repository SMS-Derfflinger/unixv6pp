use alloc::sync::Arc;
use core::{array, ptr};
use eonix_spin::Spin;

use crate::{
    constants::{fs_constants, PosixError},
    dev::{
        buffer::{Buf, Buffer, DevId, PhysicalBlock},
        buffer_manager::global_buffer_manager,
        device_manager::ROOTDEV,
    },
    fs::{
        self,
        inode::{DiskInode, Inode},
        InodeRef, SuperBlockRef,
    },
    interrupt::time::get_time,
    proc::{ProcessManager, PINOD},
    sync::{IrqGuard, SpinExt},
    user::Userspace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemError {
    NoSuchFileSystem,
    LoadSuperBlockFailed,
    NoSpace,
    BadBlock,
    BufferUnavailable,
    InodeUnavailable,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DiskSuperBlock {
    pub s_isize: i32,
    pub s_fsize: i32,

    pub s_nfree: i32,
    pub s_free: [i32; 100],

    pub s_ninode: i32,
    pub s_inode: [i32; 100],

    pub s_flock: i32,
    pub s_ilock: i32,
    pub s_fmod: i32,
    pub s_ronly: i32,
    pub s_time: i32,

    padding: [i32; 47],
}

#[derive(Clone, Copy)]
struct SleepFlag {
    locked: bool,
}

impl SleepFlag {
    const fn new() -> Self {
        Self { locked: false }
    }

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn lock(&mut self) {
        self.locked = true;
    }

    fn unlock(&mut self) {
        self.locked = false;
    }

    fn chan(&self) -> usize {
        self as *const Self as usize
    }
}

pub struct SuperBlock {
    disk: DiskSuperBlock,
    free_lock: SleepFlag,
    inode_lock: SleepFlag,
    modified: bool,
    readonly: bool,
}

impl SuperBlock {
    fn from_disk(mut disk: DiskSuperBlock, time: i32) -> Self {
        let readonly = disk.s_ronly != 0;
        let modified = disk.s_fmod != 0;

        disk.s_flock = 0;
        disk.s_ilock = 0;
        disk.s_time = time;

        Self {
            disk,
            free_lock: SleepFlag::new(),
            inode_lock: SleepFlag::new(),
            modified,
            readonly,
        }
    }

    fn to_disk(&self) -> DiskSuperBlock {
        let mut disk = self.disk;
        disk.s_flock = 0;
        disk.s_ilock = 0;
        disk.s_fmod = self.modified as i32;
        disk.s_ronly = self.readonly as i32;
        disk
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn is_flock(&self) -> bool {
        self.free_lock.is_locked()
    }

    pub fn is_ilock(&self) -> bool {
        self.inode_lock.is_locked()
    }

    fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    fn set_time(&mut self, time: i32) {
        self.disk.s_time = time;
    }

    fn free_lock_chan(&self) -> usize {
        self.free_lock.chan()
    }

    fn inode_lock_chan(&self) -> usize {
        self.inode_lock.chan()
    }

    fn lock_free_list(&mut self) {
        self.free_lock.lock();
    }

    fn unlock_free_list(&mut self) {
        self.free_lock.unlock();
    }

    fn lock_inode_list(&mut self) {
        self.inode_lock.lock();
    }

    fn unlock_inode_list(&mut self) {
        self.inode_lock.unlock();
    }
}

pub struct Mount {
    pub m_dev: DevId,
    pub m_spb: Option<SuperBlockRef>,
    pub m_inode: Option<InodeRef>,
}

impl Mount {
    pub fn new() -> Self {
        Self {
            m_dev: DevId(-1),
            m_spb: None,
            m_inode: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.m_spb.is_some() && self.m_dev.0 >= 0
    }
}

pub struct FileSystem {
    pub m_mount: [Mount; fs_constants::NMOUNT],
    updlock: bool,
}

impl FileSystem {
    fn install_loaded_super_block(&mut self, loaded_super_block: DiskSuperBlock, time: i32) {
        let super_block = SuperBlock::from_disk(loaded_super_block, time);
        self.m_mount[0].m_dev = DevId(ROOTDEV);
        self.m_mount[0].m_spb = Some(Arc::new(Spin::new(super_block)));
        self.m_mount[0].m_inode = None;
    }

    fn read_super_block() -> Result<DiskSuperBlock, FileSystemError> {
        let mut super_block = core::mem::MaybeUninit::<DiskSuperBlock>::zeroed();
        let super_block_ptr = super_block.as_mut_ptr() as *mut u8;

        for i in 0..2 {
            let buf = global_buffer_manager()
                .bread(
                    DevId(ROOTDEV),
                    PhysicalBlock((fs_constants::SUPERBLOCK_SECTOR_OFF + i) as u32),
                )
                .map_err(|_| FileSystemError::LoadSuperBlockFailed)?;

            unsafe {
                ptr::copy_nonoverlapping(
                    buf.as_bytes().as_ptr(),
                    super_block_ptr.add(i * Buf::BLOCK_SIZE),
                    Buf::BLOCK_SIZE,
                );
            }
        }

        Ok(unsafe { super_block.assume_init() })
    }

    fn write_super_block(dev: DevId, super_block: DiskSuperBlock) -> Result<(), FileSystemError> {
        let super_block_ptr = &super_block as *const DiskSuperBlock as *const u8;

        for i in 0..2 {
            let mut buf = global_buffer_manager()
                .get_blk(
                    dev,
                    PhysicalBlock((fs_constants::SUPERBLOCK_SECTOR_OFF + i) as u32),
                )
                .map_err(|_| FileSystemError::BufferUnavailable)?;

            unsafe {
                ptr::copy_nonoverlapping(
                    super_block_ptr.add(i * Buf::BLOCK_SIZE),
                    buf.as_bytes_mut().as_mut_ptr(),
                    Buf::BLOCK_SIZE,
                );
            }

            global_buffer_manager()
                .bwrite(buf)
                .map_err(|_| FileSystemError::BufferUnavailable)?;
        }

        Ok(())
    }

    fn scan_free_inodes(dev: DevId, isize: i32) -> Result<(i32, [i32; 100]), FileSystemError> {
        let mut ino = -1;
        let mut ninode = 0;
        let mut inode = [0; 100];

        for i in 0..isize {
            let buf = global_buffer_manager()
                .bread(
                    dev,
                    PhysicalBlock(fs_constants::INODE_SECTOR_OFF as u32 + i as u32),
                )
                .map_err(|_| FileSystemError::BufferUnavailable)?;

            for disk_inode in buf
                .as_slice::<DiskInode>()
                .iter()
                .take(fs_constants::INODE_NUMBER_PER_SECTOR)
            {
                ino += 1;

                if !disk_inode.d_mode.is_empty() {
                    continue;
                }

                if fs::global_inode_table().get(dev, ino).is_some() {
                    continue;
                }

                inode[ninode as usize] = ino;
                ninode += 1;

                if ninode >= 100 {
                    break;
                }
            }

            if ninode >= 100 {
                break;
            }
        }

        Ok((ninode, inode))
    }

    fn map_error_to_posix(err: FileSystemError) -> PosixError {
        match err {
            FileSystemError::NoSpace => PosixError::ENOSPC,
            FileSystemError::InodeUnavailable => PosixError::ENFILE,
            _ => PosixError::EIO,
        }
    }

    pub fn new() -> Self {
        Self {
            m_mount: array::from_fn(|_| Mount::new()),
            updlock: false,
        }
    }

    pub fn load_super_block(&mut self) -> Result<(), FileSystemError> {
        let time = get_time() as i32;
        self.install_loaded_super_block(Self::read_super_block()?, time);

        Ok(())
    }

    pub fn get_fs(&self, dev: DevId) -> Result<SuperBlockRef, FileSystemError> {
        for mount in &self.m_mount {
            if mount.m_dev != dev {
                continue;
            }

            let Some(spb) = mount.m_spb.as_ref() else {
                continue;
            };

            let mut sb = spb.lock();
            if sb.disk.s_nfree > 100 || sb.disk.s_ninode > 100 {
                sb.disk.s_nfree = 0;
                sb.disk.s_ninode = 0;
            }

            return Ok(spb.clone());
        }

        Err(FileSystemError::NoSuchFileSystem)
    }

    pub fn update(&mut self) {
        if self.updlock {
            return;
        }

        self.updlock = true;
        let time = get_time() as i32;

        for mount in &self.m_mount {
            let Some(spb) = mount.m_spb.as_ref() else {
                continue;
            };

            let should_sync = {
                let sb = spb.lock();
                sb.is_modified() && !sb.is_ilock() && !sb.is_flock() && !sb.is_readonly()
            };

            if !should_sync {
                continue;
            }

            {
                let mut sb = spb.lock();
                sb.set_modified(false);
                sb.set_time(time);
            }

            let sb = spb.lock().to_disk();
            let _ = Self::write_super_block(mount.m_dev, sb);
        }

        fs::global_inode_table().update_inode_table();

        self.updlock = false;

        let _ = global_buffer_manager().bflush(None);
    }

    pub fn i_alloc(&mut self, dev: DevId) -> Result<InodeRef, FileSystemError> {
        let spb = self.get_fs(dev)?;
        let ilock_addr = spb.lock().inode_lock_chan();

        loop {
            let mut sb = spb.lock();
            while sb.is_ilock() {
                let ctx = IrqGuard::disable_save();
                drop(sb);
                Userspace::get()
                    .proc()
                    .sleep_kernel_with_irq_guard(ilock_addr, PINOD, ctx);
                sb = spb.lock();
            }

            if sb.disk.s_ninode > 0 {
                break;
            }

            sb.lock_inode_list();
            let isize = sb.disk.s_isize;
            drop(sb);

            let refill_result = Self::scan_free_inodes(dev, isize);

            let mut sb = spb.lock();
            sb.unlock_inode_list();
            let (ninode, inode) = match refill_result {
                Ok(refill) => refill,
                Err(err) => {
                    drop(sb);
                    ProcessManager::get().wakeup_all(ilock_addr);
                    return Err(err);
                }
            };
            sb.disk.s_ninode = ninode;
            sb.disk.s_inode = inode;
            let has_free_inode = sb.disk.s_ninode > 0;
            drop(sb);

            ProcessManager::get().wakeup_all(ilock_addr);

            if !has_free_inode {
                return Err(FileSystemError::NoSpace);
            }
        }

        loop {
            let ino = {
                let mut sb = spb.lock();
                if sb.disk.s_ninode <= 0 {
                    return Err(FileSystemError::NoSpace);
                }
                sb.disk.s_ninode -= 1;
                sb.disk.s_inode[sb.disk.s_ninode as usize]
            };

            let inode = fs::global_inode_table()
                .i_get(dev, ino)
                .map_err(|_| FileSystemError::InodeUnavailable)?;

            let is_free = {
                let inode = inode.lock();
                inode.i_mode.is_empty()
            };

            if is_free {
                inode.lock().clean();
                spb.lock().set_modified(true);
                return Ok(inode.into_inner());
            }
        }
    }

    pub fn i_free(&mut self, dev: DevId, number: i32) -> Result<(), FileSystemError> {
        let spb = self.get_fs(dev)?;
        let mut sb = spb.lock();

        if sb.is_ilock() || sb.disk.s_ninode >= 100 {
            return Ok(());
        }

        let idx = sb.disk.s_ninode as usize;
        sb.disk.s_inode[idx] = number;
        sb.disk.s_ninode += 1;
        sb.set_modified(true);
        Ok(())
    }

    pub fn alloc(&mut self, dev: DevId) -> Result<Buffer, FileSystemError> {
        let spb = self.get_fs(dev)?;

        let mut sb = spb.lock();
        let flock_addr = sb.free_lock_chan();
        while sb.is_flock() {
            let ctx = IrqGuard::disable_save();
            drop(sb);
            Userspace::get()
                .proc()
                .sleep_kernel_with_irq_guard(flock_addr, PINOD, ctx);
            sb = spb.lock();
        }

        if sb.disk.s_nfree <= 0 {
            return Err(FileSystemError::NoSpace);
        }

        sb.disk.s_nfree -= 1;
        let blkno = sb.disk.s_free[sb.disk.s_nfree as usize];
        if blkno == 0 {
            sb.disk.s_nfree = 0;
            return Err(FileSystemError::NoSpace);
        }

        if self.bad_block(&sb, dev, blkno) {
            return Err(FileSystemError::BadBlock);
        }

        if sb.disk.s_nfree <= 0 {
            sb.lock_free_list();
            drop(sb);

            let refill_result = global_buffer_manager()
                .bread(dev, PhysicalBlock(blkno as u32))
                .map_err(|_| FileSystemError::BufferUnavailable)
                .map(|buf| {
                    let table = buf.as_slice::<i32>();
                    let mut sb = spb.lock();
                    sb.disk.s_nfree = table[0];
                    sb.disk.s_free.copy_from_slice(&table[1..101]);
                    sb.unlock_free_list();
                });

            if refill_result.is_err() {
                spb.lock().unlock_free_list();
            }

            ProcessManager::get().wakeup_all(flock_addr);
            refill_result?;
            sb = spb.lock();
        }

        let mut buf = global_buffer_manager()
            .get_blk(dev, PhysicalBlock(blkno as u32))
            .map_err(|_| FileSystemError::BufferUnavailable)?;
        global_buffer_manager().clr_buf(&mut buf);
        sb.set_modified(true);

        Ok(buf)
    }

    pub fn free(&mut self, dev: DevId, blkno: i32) -> Result<(), FileSystemError> {
        let spb = self.get_fs(dev)?;

        let mut sb = spb.lock();
        sb.set_modified(true);

        let flock_addr = sb.free_lock_chan();
        while sb.is_flock() {
            let ctx = IrqGuard::disable_save();
            drop(sb);
            Userspace::get()
                .proc()
                .sleep_kernel_with_irq_guard(flock_addr, PINOD, ctx);
            sb = spb.lock();
        }

        if self.bad_block(&sb, dev, blkno) {
            return Err(FileSystemError::BadBlock);
        }

        if sb.disk.s_nfree <= 0 {
            sb.disk.s_nfree = 1;
            sb.disk.s_free[0] = 0;
        }

        if sb.disk.s_nfree >= 100 {
            sb.lock_free_list();
            let nfree = sb.disk.s_nfree;
            let free = sb.disk.s_free;
            drop(sb);

            let write_result = global_buffer_manager()
                .get_blk(dev, PhysicalBlock(blkno as u32))
                .map_err(|_| FileSystemError::BufferUnavailable)
                .and_then(|mut buf| {
                    let table = buf.as_slice_mut::<i32>();
                    table[0] = nfree;
                    table[1..101].copy_from_slice(&free);

                    global_buffer_manager()
                        .bwrite(buf)
                        .map_err(|_| FileSystemError::BufferUnavailable)
                });

            sb = spb.lock();
            sb.disk.s_nfree = 0;
            sb.unlock_free_list();
            ProcessManager::get().wakeup_all(flock_addr);
            write_result?;
        }

        let idx = sb.disk.s_nfree as usize;
        sb.disk.s_free[idx] = blkno;
        sb.disk.s_nfree += 1;
        sb.set_modified(true);
        Ok(())
    }

    pub fn get_mount(&self, inode: &Inode) -> Option<&Mount> {
        self.m_mount.iter().find(|mount| {
            mount.m_inode.as_ref().is_some_and(|mount_inode| {
                let mount_inode = mount_inode.lock();
                mount_inode.i_dev == inode.i_dev && mount_inode.i_number == inode.i_number
            })
        })
    }

    pub fn bad_block(&self, _spb: &SuperBlock, _dev: DevId, _blkno: i32) -> bool {
        // TODO
        false
    }
}
