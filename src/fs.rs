use alloc::sync::Arc;
use core::ops::Deref;
use eonix_spin::{NoContext, Spin, SpinGuard};
use eonix_sync_base::LazyLock;
use file_system::FileSystem;
use open_file_manager::{InodeTable, OpenFileTable};

mod file;
mod file_manager;
mod file_system;
mod inode;
mod open_file_manager;

pub(crate) type FileRef = Arc<Spin<File>>;
pub(crate) type InodeRef = Arc<Spin<Inode>>;
pub(crate) type SuperBlockRef = Arc<Spin<SuperBlock>>;

pub(crate) struct InodeRefGuard(Option<InodeRef>);

impl InodeRefGuard {
    pub fn new(inode: InodeRef) -> Self {
        Self(Some(inode))
    }

    pub fn into_inner(mut self) -> InodeRef {
        self.0.take().expect("inode guard already consumed")
    }
}

impl Drop for InodeRefGuard {
    fn drop(&mut self) {
        if let Some(inode) = self.0.take() {
            global_inode_table().i_put(inode);
        }
    }
}

impl Deref for InodeRefGuard {
    type Target = InodeRef;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("inode guard already consumed")
    }
}

pub(crate) trait InodeRefPutExt {
    fn with_i_put(self) -> InodeRefGuard;
}

impl InodeRefPutExt for InodeRef {
    fn with_i_put(self) -> InodeRefGuard {
        InodeRefGuard::new(self)
    }
}

pub use file::{File, IOParameter, InodeRefCompat, OpenFiles};
pub use file_manager::{DirSearchMode, DirectoryEntry, FileManager, InodeRefExt};
pub use file_system::SuperBlock;
pub use inode::{Inode, InodeFlag, InodeMode};
pub use open_file_manager::{GLOBAL_INODE_TABLE, GLOBAL_OPEN_FILE_TABLE};

use crate::sync::{KernelSpinGuard, SpinExt};

static GLOBAL_OPENFILE_TABLE: LazyLock<Spin<OpenFileTable>> =
    LazyLock::new(|| Spin::new(OpenFileTable::new()));

fn global_open_file_table() -> SpinGuard<'static, OpenFileTable, NoContext> {
    GLOBAL_OPENFILE_TABLE.lock_ctx::<NoContext>()
}

fn global_inode_table() -> KernelSpinGuard<'static, InodeTable> {
    GLOBAL_INODE_TABLE.lock()
}

static GLOBAL_FILE_SYSTEM: LazyLock<Spin<FileSystem>> =
    LazyLock::new(|| Spin::new(FileSystem::new()));

pub(crate) fn global_file_system() -> SpinGuard<'static, FileSystem, NoContext> {
    GLOBAL_FILE_SYSTEM.lock_ctx::<NoContext>()
}

pub(crate) fn syscall_read() {
    file_manager::FileManager::read()
}

pub(crate) fn syscall_write() {
    file_manager::FileManager::write()
}

pub(crate) fn syscall_open() {
    file_manager::FileManager::open()
}

pub(crate) fn syscall_close() {
    file_manager::FileManager::close()
}

pub(crate) fn syscall_creat() {
    file_manager::FileManager::creat()
}

pub(crate) fn syscall_link() {
    file_manager::FileManager::link()
}

pub(crate) fn syscall_unlink() {
    file_manager::FileManager::unlink()
}

pub(crate) fn syscall_chdir() {
    file_manager::FileManager::chdir()
}

pub(crate) fn syscall_mknod() {
    file_manager::FileManager::mknod()
}

pub(crate) fn syscall_chmod() {
    file_manager::FileManager::chmod()
}

pub(crate) fn syscall_chown() {
    file_manager::FileManager::chown()
}

pub(crate) fn syscall_stat() {
    file_manager::FileManager::stat()
}

pub(crate) fn syscall_seek() {
    file_manager::FileManager::seek()
}

pub(crate) fn syscall_fstat() {
    file_manager::FileManager::fstat()
}

pub(crate) fn syscall_sync() {
    global_file_system().update();
}

pub(crate) fn syscall_dup() {
    file_manager::FileManager::dup()
}

pub(crate) fn syscall_pipe() {
    file_manager::FileManager::pipe()
}
