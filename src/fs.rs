use open_file_manager::{InodeTable, OpenFileTable};

mod file;
mod inode;
mod open_file_manager;

pub use file::{File, IOParameter, OpenFiles};
pub use inode::Inode;

// TODO: consider use lock
static mut GLOBAL_INODE_TABLE: InodeTable = InodeTable::new_const();

pub unsafe fn global_inode_table() -> &'static mut InodeTable {
    &mut GLOBAL_INODE_TABLE
}

static mut GLOBAL_OPENFILE_TALBE: OpenFileTable = OpenFileTable::new_const();

pub unsafe fn global_openfile_table() -> &'static mut OpenFileTable {
    &mut GLOBAL_OPENFILE_TALBE
}
