use core::{cmp::min, ffi::CStr, ptr};

use crate::{
    compat::compat_get_time,
    constants::{PosixError, Signal},
    dev::{
        buffer::{Buffer, DevId, LogicalBlock, PhysicalBlock},
        buffer_manager::{global_buffer_manager, PPIPE},
        device_manager::ROOTDEV,
    },
    fs::{
        self,
        file::FileFlags,
        file_system::FileSystem,
        inode::{InodeFlag, InodeMode},
        File, FileRef, Inode, InodeRef, InodeRefGuard,
    },
    proc::{Channel, ProcessManager},
    sync::{IrqGuard, SpinExt},
    user::Userspace,
};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirSearchMode {
    Open = 0,
    Create = 1,
    Delete = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DirectoryEntry {
    pub m_ino: i32,
    pub m_name: [u8; 28],
}

#[repr(C)]
pub struct FileManager;

impl DirectoryEntry {
    pub const DIRSIZ: usize = 28;

    pub const fn new() -> Self {
        Self {
            m_ino: 0,
            m_name: [0; Self::DIRSIZ],
        }
    }

    pub fn name(&self) -> &[u8] {
        let end = self
            .m_name
            .iter()
            .position(|&s| s == 0)
            .expect("Invalid string");
        &self.m_name[..end]
    }
}

fn args() -> &'static mut [usize; 5] {
    &mut Userspace::get().args
}

fn set_error(err: PosixError) {
    Userspace::get().error = Some(err);
}

fn is_root() -> bool {
    Userspace::get().is_root()
}

fn i_put(inode: InodeRef) {
    drop(inode);
}

fn i_get(dev: DevId, ino: i32) -> Result<InodeRefGuard, PosixError> {
    fs::global_inode_table().i_get(dev, ino)
}

fn access_inode(inode: &Inode, mode: InodeMode) -> bool {
    if mode == InodeMode::IWRITE {
        let Ok(spb) = fs::global_file_system().get_fs(inode.i_dev) else {
            set_error(PosixError::EIO);
            return false;
        };

        if spb.lock().is_readonly() {
            set_error(PosixError::EROFS);
            return false;
        }
    }

    if Userspace::get().uid == 0 {
        let exec_bits = InodeMode::IEXEC.bits()
            | (InodeMode::IEXEC.bits() >> 3)
            | (InodeMode::IEXEC.bits() >> 6);
        let has_exec = (inode.i_mode.bits() & exec_bits) != 0;

        if mode == InodeMode::IEXEC && !has_exec {
            set_error(PosixError::EACCES);
            return false;
        }

        return true;
    }

    let mut mode_bits = mode.bits();
    if Userspace::get().uid != inode.i_uid {
        mode_bits >>= 3;
        if Userspace::get().gid != inode.i_gid {
            mode_bits >>= 3;
        }
    }

    if (inode.i_mode.bits() & mode_bits) != 0 {
        return true;
    }

    set_error(PosixError::EACCES);
    false
}

fn read_i(inode: &mut Inode) {
    let iop = Userspace::get().ioparam;
    let buffer = unsafe { core::slice::from_raw_parts_mut(iop.m_base as *mut u8, iop.m_count) };

    match inode.read(buffer, iop.m_offset) {
        Ok(nread) => {
            let iop = &mut Userspace::get().ioparam;
            iop.m_count -= nread;
            iop.m_base += nread;
            iop.m_offset += nread;
        }
        Err(err) => set_error(err),
    }
}

fn write_i(inode: &mut Inode) {
    let iop = Userspace::get().ioparam;
    let buffer = unsafe { core::slice::from_raw_parts(iop.m_base as *const u8, iop.m_count) };

    match inode.write(buffer, iop.m_offset) {
        Ok(nwrite) => {
            let iop = &mut Userspace::get().ioparam;
            iop.m_count -= nwrite;
            iop.m_base += nwrite;
            iop.m_offset += nwrite;
        }
        Err(err) => set_error(err),
    }
}

pub trait InodeRefExt {
    fn has_access(&self, mode: InodeMode) -> bool;
    fn is_regular(&self) -> bool;
    fn readblk(&self, lbn: LogicalBlock) -> Buffer;
    fn search(
        &self,
        name: &[u8],
        create: bool,
        remove: bool,
    ) -> Result<Option<InodeRefGuard>, PosixError>;
}

impl InodeRefExt for InodeRef {
    fn has_access(&self, mode: InodeMode) -> bool {
        access_inode(&self.lock(), mode)
    }

    fn is_regular(&self) -> bool {
        !self.lock().i_mode.contains(InodeMode::IFMT)
    }

    fn readblk(&self, lbn: LogicalBlock) -> Buffer {
        let mut ra = None;
        self.lock().get_blk(lbn, &mut ra).unwrap()
    }

    fn search(
        &self,
        name: &[u8],
        create: bool,
        remove: bool,
    ) -> Result<Option<InodeRefGuard>, PosixError> {
        const DENTRY_SIZE: usize = size_of::<DirectoryEntry>();
        let mut count = self.lock().i_size as usize / DENTRY_SIZE;
        let mut offset = 0;
        let mut free_offset = None;

        let mut buffer = self.readblk(LogicalBlock(0));
        while count != 0 {
            let blkoff = offset % Inode::BLOCK_SIZE;
            let idx = blkoff / DENTRY_SIZE;

            if offset != 0 && blkoff == 0 {
                buffer = self.readblk(LogicalBlock((offset / Inode::BLOCK_SIZE) as u32));
            }

            let dentry = &buffer.as_slice::<DirectoryEntry>()[idx];

            if dentry.m_ino == 0 {
                free_offset.get_or_insert(offset);
            }

            if dentry.m_ino != 0 && dentry.name() == name {
                Userspace::get().dentry = *dentry;
                break;
            }

            offset += DENTRY_SIZE;
            count -= 1;
        }

        let mut ret = None;
        let mut err = None;
        loop {
            if count == 0 {
                if !create {
                    err = Some(PosixError::ENOENT);
                    break;
                }

                if !self.has_access(InodeMode::IWRITE) {
                    err = Some(PosixError::EACCES);
                    break;
                }

                Userspace::get().set_cwd_parent(self.clone());

                if free_offset.is_none() {
                    let _ = free_offset.insert(offset);
                    self.lock().i_flag.insert(InodeFlag::IUPD);
                }

                offset = free_offset.unwrap();
                break;
            }

            if remove {
                if !self.has_access(InodeMode::IWRITE) {
                    err = Some(PosixError::EACCES);
                }
                break;
            }

            let dev = self.lock().i_dev;
            let ino = Userspace::get().dentry.m_ino;

            match i_get(dev, ino) {
                Err(e) => err = Some(e),
                Ok(iref) => ret = Some(iref),
            }

            break;
        }

        Userspace::get().ioparam.m_offset = offset;
        Userspace::get().ioparam.m_count = count;

        match (err, ret) {
            (Some(err), _) => Err(err),
            (None, ret) => Ok(ret),
        }
    }
}

impl FileManager {
    fn mode(&self, mode: u32) -> DirSearchMode {
        match mode {
            1 => DirSearchMode::Create,
            2 => DirSearchMode::Delete,
            _ => DirSearchMode::Open,
        }
    }

    pub fn find(
        &self,
        mut path: &[u8],
        mode: DirSearchMode,
    ) -> Result<Option<InodeRefGuard>, PosixError> {
        let mut iref = if let Some(b'/') = path.first() {
            i_get(DevId(ROOTDEV), FileSystem::ROOTINO)?
        } else {
            InodeRefGuard::new(Userspace::get().getcwd())
        };

        while let Some(b'/') = path.first() {
            path = &path[1..];
        }

        while Userspace::get().error.is_none() && !path.is_empty() {
            if (iref.lock().i_mode & InodeMode::IFMT) != InodeMode::IFDIR {
                return Err(PosixError::ENOTDIR);
            }

            if !iref.has_access(InodeMode::IEXEC) {
                return Err(PosixError::EACCES);
            }

            let next_idx = path.iter().position(|&c| c == b'/').unwrap_or(path.len());
            let name = &path[..next_idx];
            path = &path[next_idx..];

            if name.len() >= DirectoryEntry::DIRSIZ {
                return Err(PosixError::EINVAL);
            }

            {
                let dbuf = Userspace::get().argdir_mut();
                dbuf[..name.len()].copy_from_slice(name);
                dbuf[name.len()..].fill(0);
            }

            while let Some(b'/') = path.first() {
                path = &path[1..];
            }

            if let Some(i) = iref.search(
                name,
                mode == DirSearchMode::Create && path.is_empty(),
                mode == DirSearchMode::Delete && path.is_empty(),
            )? {
                iref = i;
            } else {
                return Ok(None);
            }
        }

        Ok(Some(iref))
    }

    fn readp_inner(&self, file_ref: &FileRef) {
        loop {
            let (inode_ref, foff) = {
                let file = file_ref.lock();
                (
                    file.f_inode.as_ref().expect("pipe without inode").clone(),
                    file.f_offset,
                )
            };
            let mut inode = Inode::lock_pipe(&inode_ref);

            if foff == inode.i_size as i32 {
                if foff != 0 {
                    {
                        let mut file = file_ref.lock();
                        file.f_offset = 0;
                    }
                    inode.i_size = 0;

                    if inode.i_mode.contains(InodeMode::IWRITE) {
                        inode.i_mode.remove(InodeMode::IWRITE);
                        let chan = inode.channel_write().channel_addr();
                        ProcessManager::get().wakeup_all(chan);
                    }
                }

                inode.prele();
                drop(inode);

                if inode_ref.lock().i_count < 2 {
                    return;
                }

                let mut inode = inode_ref.lock();
                inode.i_mode.insert(InodeMode::IREAD);
                let chan = inode.channel_read().channel_addr();
                let ctx = IrqGuard::disable_save();
                drop(inode);
                Userspace::get()
                    .proc()
                    .sleep_user_with_irq_guard(chan, PPIPE, ctx);
                continue;
            }

            Userspace::get().ioparam.m_offset = foff as usize;
            read_i(&mut inode);
            let new_off = Userspace::get().ioparam.m_offset as i32;
            {
                let mut file = file_ref.lock();
                file.f_offset = new_off;
            }
            inode.prele();
            return;
        }
    }

    fn writep_inner(&self, file_ref: &FileRef) {
        let mut count = Userspace::get().ioparam.m_count as i32;

        loop {
            let inode_ref = {
                let file = file_ref.lock();
                file.f_inode.as_ref().expect("pipe without inode").clone()
            };
            let mut inode = Inode::lock_pipe(&inode_ref);

            if count == 0 {
                inode.prele();
                Userspace::get().ioparam.m_count = 0;
                return;
            }

            if inode.i_count < 2 {
                inode.prele();
                set_error(PosixError::EPIPE);
                Userspace::get().proc().raise(Signal::SIGPIPE);
                return;
            }

            if inode.i_size as usize == Inode::PIPSIZ {
                inode.i_mode.insert(InodeMode::IWRITE);
                let chan = inode.channel_write().channel_addr();
                let ctx = IrqGuard::disable_save();
                inode.prele();
                Userspace::get()
                    .proc()
                    .sleep_user_with_irq_guard(chan, PPIPE, ctx);
                continue;
            }

            Userspace::get().ioparam.m_offset = inode.i_size as usize;
            Userspace::get().ioparam.m_count = min(
                count as usize,
                Inode::PIPSIZ.saturating_sub(Userspace::get().ioparam.m_offset),
            );
            count -= Userspace::get().ioparam.m_count as i32;

            write_i(&mut inode);

            let wake_read = inode.i_mode.contains(InodeMode::IREAD);
            if wake_read {
                inode.i_mode.remove(InodeMode::IREAD);
            }
            let chan = inode.channel_read().channel_addr();
            inode.prele();

            if wake_read {
                ProcessManager::get().wakeup_all(chan);
            }
        }
    }

    pub fn open1(&self, pinode: InodeRef, mode: i32, trf: i32) {
        if trf != 2 {
            if (mode & FileFlags::FREAD.bits() as i32) != 0
                && self.access(&pinode, InodeMode::IREAD.bits())
            {
                i_put(pinode);
                return;
            }

            if (mode & FileFlags::FWRITE.bits() as i32) != 0 {
                if self.access(&pinode, InodeMode::IWRITE.bits()) {
                    i_put(pinode);
                    return;
                }

                if (pinode.lock().i_mode & InodeMode::IFMT) == InodeMode::IFDIR {
                    set_error(PosixError::EISDIR);
                    i_put(pinode);
                    return;
                }
            }
        }

        if trf == 1 {
            pinode.lock().release();
        }

        pinode.lock().prele();

        let (fd, fileref) =
            match fs::global_open_file_table().f_alloc(&mut Userspace::get().open_files) {
                Ok(v) => v,
                Err(err) => {
                    set_error(err);
                    i_put(pinode);
                    return;
                }
            };
        Userspace::get().set_user_retval(fd as u32);

        {
            let mut file = fileref.lock();
            file.f_flag =
                FileFlags::from_bits_truncate(mode as u32) & (FileFlags::FREAD | FileFlags::FWRITE);
            file.f_inode = Some(pinode.clone());
        }

        if let Err(err) = pinode
            .lock()
            .open_i((mode as u32) & FileFlags::FWRITE.bits())
        {
            set_error(err);
        }

        if Userspace::get().error.is_some() {
            Userspace::get().open_files.clear_f(fd);
            let mut file = fileref.lock();
            file.f_inode = None;
            i_put(pinode);
        }
    }

    pub fn stat1(&self, pinode: &InodeRef, stat_buf: usize) {
        let inode = pinode.lock();
        let ino = inode.i_number;
        let dev = inode.i_dev;
        inode.i_update(compat_get_time() as i32);
        drop(inode);

        let sector = FileSystem::INODE_ZONE_START_SECTOR as u32
            + ino as u32 / FileSystem::INODE_NUMBER_PER_SECTOR as u32;

        let buf = match global_buffer_manager().bread(dev, PhysicalBlock(sector)) {
            Ok(buf) => buf,
            Err(err) => {
                set_error(err.into());
                return;
            }
        };

        const DISK_INODE_SIZE: usize = 64;
        let off = (ino as usize % FileSystem::INODE_NUMBER_PER_SECTOR) * DISK_INODE_SIZE;
        unsafe {
            ptr::copy_nonoverlapping(
                buf.as_slice::<u8>().as_ptr().add(off),
                stat_buf as *mut u8,
                DISK_INODE_SIZE,
            );
        }
    }

    pub fn namei(&self, mode: u32) -> Option<InodeRef> {
        let path = Userspace::get().argdir();
        match self.find(path, self.mode(mode)) {
            Ok(Some(iref)) => Some(iref.into_inner()),
            Ok(None) => None,
            Err(err) => {
                set_error(err);
                None
            }
        }
    }

    pub fn maknode(&self, mode: u32) -> Option<InodeRef> {
        let parent = Userspace::get().cwd_parent.as_ref()?.clone();
        let dev = parent.lock().i_dev;

        let inode = match fs::global_file_system().i_alloc(dev) {
            Ok(inode) => inode,
            Err(_) => {
                set_error(PosixError::ENOSPC);
                return None;
            }
        };

        {
            let mut inode_l = inode.lock();
            inode_l.i_flag.insert(InodeFlag::IACC | InodeFlag::IUPD);
            inode_l.i_mode = InodeMode::from_bits_retain(mode) | InodeMode::IALLOC;
            inode_l.i_nlink = 1;
            inode_l.i_uid = Userspace::get().uid;
            inode_l.i_gid = Userspace::get().gid;
        }

        self.writedir(&inode);
        Some(inode)
    }

    pub fn writedir(&self, pinode: &InodeRef) {
        Userspace::get().dentry.m_ino = pinode.lock().i_number;

        for i in 0..DirectoryEntry::DIRSIZ {
            Userspace::get().dentry.m_name[i] = Userspace::get().argdir_mut()[i];
        }

        Userspace::get().ioparam.m_count = DirectoryEntry::DIRSIZ + 4;
        Userspace::get().ioparam.m_base = (&raw mut Userspace::get().dentry) as usize;

        let Some(parent) = Userspace::get().cwd_parent.as_ref().cloned() else {
            return;
        };

        let mut parent = parent.lock();
        write_i(&mut parent);
        drop(parent);
        i_put(
            Userspace::get()
                .cwd_parent
                .take()
                .expect("cwd parent missing"),
        );
    }

    pub fn access(&self, pinode: &InodeRef, mode: u32) -> bool {
        !access_inode(&pinode.lock(), InodeMode::from_bits_retain(mode))
    }

    pub fn owner(&self) -> Option<InodeRef> {
        let inode = self.namei(DirSearchMode::Open as u32)?;

        let uid = inode.lock().i_uid;
        if Userspace::get().uid == uid || is_root() {
            return Some(inode);
        }

        i_put(inode);
        None
    }
}

impl FileManager {
    pub fn open() {
        let Some(inode) = FileManager.namei(DirSearchMode::Open as u32) else {
            return;
        };

        FileManager.open1(inode, args()[1] as i32, 0);
    }

    pub fn creat() {
        let new_acc_mode =
            (args()[1] as u32) & (InodeMode::IRWXU | InodeMode::IRWXG | InodeMode::IRWXO).bits();

        match FileManager.namei(DirSearchMode::Create as u32) {
            None => {
                if Userspace::get().error.is_some() {
                    return;
                }

                let Some(inode) = FileManager.maknode(new_acc_mode & !InodeMode::ISVTX.bits())
                else {
                    return;
                };

                FileManager.open1(inode, FileFlags::FWRITE.bits() as i32, 2);
            }
            Some(inode) => {
                FileManager.open1(inode.clone(), FileFlags::FWRITE.bits() as i32, 1);
                inode
                    .lock()
                    .i_mode
                    .insert(InodeMode::from_bits_retain(new_acc_mode));
            }
        }
    }

    pub fn close() {
        let fd = args()[0];

        let file_ref = match Userspace::get().open_files.get_f(fd) {
            Ok(file) => file,
            Err(err) => {
                set_error(err.into());
                return;
            }
        };

        Userspace::get().open_files.clear_f(fd);
        drop(file_ref);
    }

    pub fn seek() {
        let fd = args()[0];
        let file_ref = match Userspace::get().open_files.get_f(fd) {
            Ok(file) => file,
            Err(err) => {
                set_error(err);
                return;
            }
        };

        let mut file = file_ref.lock();
        if file.f_flag.contains(FileFlags::FPIPE) {
            set_error(PosixError::ESPIPE);
            return;
        }

        let mut offset = args()[1] as i32;
        let mut whence = args()[2] as i32;
        if whence > 2 {
            offset <<= 9;
            whence -= 3;
        }

        match whence {
            0 => file.f_offset = offset,
            1 => file.f_offset += offset,
            2 => {
                let inode = file.f_inode.as_ref().expect("file without inode").clone();
                file.f_offset = inode.lock().i_size as i32 + offset;
            }
            _ => {}
        }
    }

    pub fn dup() {
        let fd = args()[0];

        let file_ref = match Userspace::get().open_files.get_f(fd) {
            Ok(file) => file,
            Err(err) => {
                set_error(err);
                return;
            }
        };

        let new_fd = match Userspace::get().open_files.alloc_free_slot() {
            Ok(fd) => fd,
            Err(err) => {
                set_error(err);
                return;
            }
        };

        Userspace::get().open_files.set_f(new_fd, file_ref.clone());
    }

    pub fn fstat() {
        let fd = args()[0];

        let file_ref = match Userspace::get().open_files.get_f(fd) {
            Ok(file) => file,
            Err(err) => {
                set_error(err);
                return;
            }
        };

        let inode = file_ref
            .lock()
            .f_inode
            .as_ref()
            .expect("file without inode")
            .clone();
        FileManager.stat1(&inode, args()[1]);
    }

    pub fn stat() {
        let Some(inode) = FileManager.namei(DirSearchMode::Open as u32) else {
            return;
        };

        FileManager.stat1(&inode, args()[1]);
        i_put(inode);
    }

    pub fn read() {
        FileManager::rdwr(FileFlags::FREAD.bits());
    }

    pub fn write() {
        FileManager::rdwr(FileFlags::FWRITE.bits());
    }

    pub fn rdwr(mode: u32) {
        let fd = args()[0];
        let count = args()[2];

        let file_ref = match Userspace::get().open_files.get_f(fd) {
            Ok(file) => file,
            Err(err) => {
                set_error(err);
                return;
            }
        };

        {
            let file = file_ref.lock();
            let mode = FileFlags::from_bits_truncate(mode);
            if !file.f_flag.contains(mode) {
                set_error(PosixError::EACCES);
                return;
            }
        }

        Userspace::get().ioparam.m_base = args()[1];
        Userspace::get().ioparam.m_count = count;

        let is_pipe = file_ref.lock().f_flag.contains(FileFlags::FPIPE);
        if is_pipe {
            if mode == FileFlags::FREAD.bits() {
                FileManager.readp_inner(&file_ref);
            } else {
                FileManager.writep_inner(&file_ref);
            }
        } else {
            let (inode_ref, foff) = {
                let file = file_ref.lock();
                (
                    file.f_inode.as_ref().expect("file without inode").clone(),
                    file.f_offset,
                )
            };

            let mut inode = Inode::lock_file(&inode_ref);
            Userspace::get().ioparam.m_offset = foff as usize;
            if mode == FileFlags::FREAD.bits() {
                read_i(&mut inode);
            } else {
                write_i(&mut inode);
            }
            inode.nf_rele();

            let moved = count.saturating_sub(Userspace::get().ioparam.m_count);
            let mut file = file_ref.lock();
            file.f_offset += moved as i32;
        }

        Userspace::get()
            .set_user_retval((count.saturating_sub(Userspace::get().ioparam.m_count)) as u32);
    }

    pub fn pipe() {
        let inode_ref = match fs::global_file_system().i_alloc(DevId(ROOTDEV)) {
            Ok(inode) => inode,
            Err(_) => {
                set_error(PosixError::ENOSPC);
                return;
            }
        };
        let (fd_r, file_r) =
            match fs::global_open_file_table().f_alloc(&mut Userspace::get().open_files) {
                Ok(v) => v,
                Err(err) => {
                    set_error(err);
                    i_put(inode_ref);
                    return;
                }
            };

        let (fd_w, file_w) =
            match fs::global_open_file_table().f_alloc(&mut Userspace::get().open_files) {
                Ok(v) => v,
                Err(err) => {
                    set_error(err);
                    Userspace::get().open_files.clear_f(fd_r);
                    i_put(inode_ref);
                    return;
                }
            };

        let fdarr = args()[0] as *mut i32;
        unsafe {
            fdarr.write(fd_r as i32);
            fdarr.add(1).write(fd_w as i32);
        }

        {
            let mut fr = file_r.lock();
            fr.f_flag = FileFlags::FREAD | FileFlags::FPIPE;
            fr.f_inode = Some(inode_ref.clone());
        }
        {
            let mut fw = file_w.lock();
            fw.f_flag = FileFlags::FWRITE | FileFlags::FPIPE;
            fw.f_inode = Some(inode_ref.clone());
        }

        let mut inode = inode_ref.lock();
        inode.i_flag = InodeFlag::IACC | InodeFlag::IUPD;
        inode.i_mode = InodeMode::IALLOC;
    }

    pub fn readp(_file: *mut File) {
        // Compatibility entrypoint is currently unused; rdwr() calls Rust-native implementation.
    }

    pub fn writep(_file: *mut File) {
        // Compatibility entrypoint is currently unused; rdwr() calls Rust-native implementation.
    }

    pub fn setcurdir(pathname: usize) {
        let path = unsafe { CStr::from_ptr(pathname as *const i8) }.to_bytes();
        let curdir = &mut Userspace::get().cwd_full;

        if path.first().copied() != Some(b'/') {
            let mut len = curdir.iter().position(|&x| x == 0).unwrap_or(curdir.len());
            if len > 0 && curdir[len - 1] != b'/' {
                if len < curdir.len() {
                    curdir[len] = b'/';
                    len += 1;
                }
            }

            let copy_len = min(path.len(), curdir.len().saturating_sub(len + 1));
            curdir[len..len + copy_len].copy_from_slice(&path[..copy_len]);
            if len + copy_len < curdir.len() {
                curdir[len + copy_len] = 0;
            }
        } else {
            curdir.fill(0);
            let copy_len = min(path.len(), curdir.len().saturating_sub(1));
            curdir[..copy_len].copy_from_slice(&path[..copy_len]);
            curdir[copy_len] = 0;
        }
    }

    pub fn chmod() {
        let mode = args()[1] as u32;

        let Some(iref) = FileManager.owner() else {
            return;
        };

        let mut inode = iref.lock();
        inode.i_mode &= !InodeMode::from_bits_retain(0xFFF);
        inode.i_mode |= InodeMode::from_bits_retain(mode & 0xFFF);
        inode.i_flag.insert(InodeFlag::IUPD);
        drop(inode);

        i_put(iref);
    }

    pub fn chown() {
        if !is_root() {
            return;
        }

        let Some(iref) = FileManager.owner() else {
            return;
        };

        let mut inode = iref.lock();
        inode.i_uid = args()[1] as u16;
        inode.i_gid = args()[2] as u16;
        inode.i_flag.insert(InodeFlag::IUPD);
        drop(inode);

        i_put(iref);
    }

    pub fn chdir() {
        let Some(inode) = FileManager.namei(DirSearchMode::Open as u32) else {
            return;
        };

        {
            let inod = inode.lock();
            if (inod.i_mode & InodeMode::IFMT) != InodeMode::IFDIR {
                set_error(PosixError::ENOTDIR);
                drop(inod);
                i_put(inode);
                return;
            }
        }

        if FileManager.access(&inode, InodeMode::IEXEC.bits()) {
            i_put(inode);
            return;
        }

        if let Some(old) = Userspace::get().cwd.take() {
            i_put(old);
        }
        Userspace::get().cwd = Some(inode.clone());

        inode.lock().prele();
        FileManager::setcurdir(args()[0]);
    }

    pub fn link() {
        let Some(inode) = FileManager.namei(DirSearchMode::Open as u32) else {
            return;
        };

        {
            let mut i = inode.lock();
            if i.i_nlink >= 255 {
                set_error(PosixError::EMLINK);
                drop(i);
                i_put(inode);
                return;
            }

            if (i.i_mode & InodeMode::IFMT) == InodeMode::IFDIR && !is_root() {
                drop(i);
                i_put(inode);
                return;
            }

            i.i_flag.remove(InodeFlag::ILOCK);
        }

        let old_dirp = Userspace::get().dirp;
        Userspace::get().dirp = args()[1] as *mut u8;
        let new_inode = FileManager.namei(DirSearchMode::Create as u32);
        Userspace::get().dirp = old_dirp;

        if let Some(new_inode) = new_inode {
            set_error(PosixError::EEXIST);
            i_put(new_inode);
        }

        if Userspace::get().error.is_some() {
            i_put(inode);
            return;
        }

        let Some(parent) = Userspace::get().cwd_parent.as_ref().cloned() else {
            i_put(inode);
            return;
        };

        if parent.lock().i_dev != inode.lock().i_dev {
            i_put(parent);
            set_error(PosixError::EXDEV);
            i_put(inode);
            return;
        }

        FileManager.writedir(&inode);
        {
            let mut i = inode.lock();
            i.i_nlink += 1;
            i.i_flag.insert(InodeFlag::IUPD);
        }
        i_put(inode);
    }

    pub fn unlink() {
        let Some(d_inode) = FileManager.namei(DirSearchMode::Delete as u32) else {
            return;
        };

        d_inode.lock().prele();

        let dev = d_inode.lock().i_dev;
        let ino = Userspace::get().dentry.m_ino;

        let inode = match i_get(dev, ino) {
            Ok(i) => i,
            Err(_) => {
                set_error(PosixError::EIO);
                i_put(d_inode);
                return;
            }
        };

        if (inode.lock().i_mode & InodeMode::IFMT) == InodeMode::IFDIR && !is_root() {
            i_put(d_inode);
            return;
        }

        Userspace::get().ioparam.m_offset -= DirectoryEntry::DIRSIZ + 4;
        Userspace::get().ioparam.m_base = (&raw mut Userspace::get().dentry) as usize;
        Userspace::get().ioparam.m_count = DirectoryEntry::DIRSIZ + 4;
        Userspace::get().dentry.m_ino = 0;
        write_i(&mut d_inode.lock());

        {
            let mut inode_l = inode.lock();
            inode_l.i_nlink -= 1;
            inode_l.i_flag.insert(InodeFlag::IUPD);
        }

        i_put(d_inode);
    }

    pub fn mknod() {
        if !is_root() {
            set_error(PosixError::EPERM);
            return;
        }

        if let Some(inode) = FileManager.namei(DirSearchMode::Create as u32) {
            set_error(PosixError::EEXIST);
            i_put(inode);
            return;
        }

        if Userspace::get().error.is_some() {
            return;
        }

        let Some(inode) = FileManager.maknode(args()[1] as u32) else {
            return;
        };

        let mut inode_l = inode.lock();
        if inode_l
            .i_mode
            .intersects(InodeMode::IFBLK | InodeMode::IFCHR)
        {
            inode_l.i_addr[0].0 = args()[2] as u32;
        }
        drop(inode_l);

        i_put(inode);
    }
}
