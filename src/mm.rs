mod allocator;
mod page;
mod page_manager;
mod swapper_manager;
mod zone;

pub use allocator::{phys_to_virt, virt_to_phys};
pub use page::{PageList, PhysPage, PAGE_SIZE};
pub use page_manager::{KERNEL_PAGE_MANAGER, USER_PAGE_MANAGER, alloc_page, free_page};
pub use zone::ZONE;
