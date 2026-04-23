use buddy_allocator::{BuddyAllocator, BuddyFolio};
use eonix_mm::{
    address::{Addr, PAddr, PRange},
    paging::{Folio, Zone, PFN},
};
use eonix_spin::Spin;

use crate::{
    mm::{
        zone::{init_zone, MemoryZone, MEM_SIZE, ZONE},
        PageList, PhysPage,
    },
    sync::SpinExt as _,
};

impl BuddyFolio for PhysPage {
    fn pfn(&self) -> PFN {
        Folio::pfn(self)
    }

    fn get_order(&self) -> u32 {
        self.order()
    }

    fn is_buddy(&self) -> bool {
        self.is_buddy
    }

    fn set_order(&mut self, order: u32) {
        self.order = order as u8;
    }

    fn set_buddy(&mut self, value: bool) {
        self.is_buddy = value;
    }
}

pub static KERNEL_PAGE_MANAGER: Spin<BuddyAllocator<MemoryZone, PageList>> =
    Spin::new(BuddyAllocator::new(&ZONE));

pub static USER_PAGE_MANAGER: Spin<BuddyAllocator<MemoryZone, PageList>> =
    Spin::new(BuddyAllocator::new(&ZONE));

pub fn init_page_managers() {
    let krange = PRange::new(PAddr::from_val(0x204000), PAddr::from_val(0x3ff000));
    let urange = PRange::new(PAddr::from_val(0x400000), PAddr::from_val(MEM_SIZE));

    init_zone();

    {
        let mut kpm = KERNEL_PAGE_MANAGER.lock();
        kpm.create_folios(krange.start(), krange.end());
    }

    {
        let mut upm = USER_PAGE_MANAGER.lock();
        upm.create_folios(urange.start(), urange.end());
    }
}

pub fn alloc_kernel_page(size: usize) -> &'static mut PhysPage {
    let mut allocator = KERNEL_PAGE_MANAGER.lock();

    let aligned_size = size.next_power_of_two();
    let order = aligned_size.trailing_zeros() - 12;

    allocator.alloc_order(order).expect("Out of memory")
}

/// Allocate a page and return its physical address
#[no_mangle]
pub extern "C" fn alloc_page(size: usize, user: bool) -> usize {
    let mut allocator = if user {
        USER_PAGE_MANAGER.lock()
    } else {
        KERNEL_PAGE_MANAGER.lock()
    };

    let aligned_size = size.next_power_of_two();
    let order = aligned_size.trailing_zeros() - 12;

    let page = allocator.alloc_order(order).expect("Out of memory");

    #[cfg(trace_alloc)]
    println_trace!("Allocated {:?} size={:#x}", page.pfn(), size);

    page.phys().addr()
}

#[no_mangle]
pub extern "C" fn free_page(addr: usize, size: usize, user: bool) {
    if size == 0 {
        return;
    }

    let paddr = PAddr::from_val(addr);

    let mut allocator = if user {
        USER_PAGE_MANAGER.lock()
    } else {
        KERNEL_PAGE_MANAGER.lock()
    };

    let mut page_ptr = ZONE.get_page(PFN::from(paddr)).expect("Bad address");
    let page = unsafe {
        // SAFETY: We've got the page pointer from MemoryZone.
        page_ptr.as_mut()
    };

    assert!(size <= (1 << (page.order as usize + 12)), "Wrong size");

    #[cfg(trace_alloc)]
    println_trace!("Deallocate {:?} size={:#x}", page.pfn(), size);

    unsafe {
        allocator.dealloc(page);
    }
}
