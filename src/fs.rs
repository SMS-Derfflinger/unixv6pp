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
pub use inode::{inoderef_leak, Inode, InodeMode};
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

// TODO
fn with_file_manager(f: extern "C" fn()) {
    f();
}

pub(crate) fn syscall_read() {
    with_file_manager(file_manager::FileManager_read);
}

pub(crate) fn syscall_write() {
    with_file_manager(file_manager::FileManager_write);
}

pub(crate) fn syscall_open() {
    with_file_manager(file_manager::FileManager_open);
}

pub(crate) fn syscall_close() {
    with_file_manager(file_manager::FileManager_close);
}

pub(crate) fn syscall_creat() {
    with_file_manager(file_manager::FileManager_creat);
}

pub(crate) fn syscall_link() {
    with_file_manager(file_manager::FileManager_link);
}

pub(crate) fn syscall_unlink() {
    with_file_manager(file_manager::FileManager_unlink);
}

pub(crate) fn syscall_chdir() {
    with_file_manager(file_manager::FileManager_chdir);
}

pub(crate) fn syscall_mknod() {
    with_file_manager(file_manager::FileManager_mknod);
}

pub(crate) fn syscall_chmod() {
    with_file_manager(file_manager::FileManager_chmod);
}

pub(crate) fn syscall_chown() {
    with_file_manager(file_manager::FileManager_chown);
}

pub(crate) fn syscall_stat() {
    with_file_manager(file_manager::FileManager_stat);
}

pub(crate) fn syscall_seek() {
    with_file_manager(file_manager::FileManager_seek);
}

pub(crate) fn syscall_fstat() {
    with_file_manager(file_manager::FileManager_fstat);
}

pub(crate) fn syscall_sync() {
    global_file_system().update();
}

pub(crate) fn syscall_dup() {
    with_file_manager(file_manager::FileManager_dup);
}

pub(crate) fn syscall_pipe() {
    with_file_manager(file_manager::FileManager_pipe);
}
