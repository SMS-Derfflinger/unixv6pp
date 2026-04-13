use eonix_spin::{NoContext, Spin, SpinGuard};
use eonix_sync_base::LazyLock;
use open_file_manager::{InodeTable, OpenFileTable};

mod file;
mod file_system;
mod inode;
mod open_file_manager;

pub use file::{File, IOParameter, OpenFiles};
pub use inode::Inode;

static GLOBAL_OPENFILE_TABLE: LazyLock<Spin<OpenFileTable>> =
    LazyLock::new(|| Spin::new(OpenFileTable::new()));

static GLOBAL_INODE_TABLE: LazyLock<Spin<InodeTable>> =
    LazyLock::new(|| Spin::new(InodeTable::new()));

fn global_open_file_table() -> SpinGuard<'static, OpenFileTable, NoContext> {
    GLOBAL_OPENFILE_TABLE.lock_ctx::<NoContext>()
}

fn global_inode_table() -> SpinGuard<'static, InodeTable, NoContext> {
    GLOBAL_INODE_TABLE.lock_ctx::<NoContext>()
}
