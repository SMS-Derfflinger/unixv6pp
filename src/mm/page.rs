use eonix_mm::{
    address::{Addr, PAddr, PRange},
    paging::{Folio, FolioList, FolioListSized, PFN, Zone},
};
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListAtomicLink, UnsafeRef};

use crate::mm::zone::{MemoryZone, ZONE};

pub const PAGE_SIZE: usize = 0x1000;

pub struct PhysPage {
    pub link: LinkedListAtomicLink,
    pub order: u8,
    pub is_slab: bool,
    pub is_buddy: bool,
}

intrusive_adapter!(PagesAdapter = UnsafeRef<PhysPage>: PhysPage { link: LinkedListAtomicLink });
pub struct PageList(LinkedList<PagesAdapter>);

impl PhysPage {
    pub const fn new() -> Self {
        Self {
            link: LinkedListAtomicLink::new(),
            order: 0,
            is_slab: false,
            is_buddy: false,
        }
    }

    pub fn phys(&self) -> PAddr {
        PAddr::from(self.pfn())
    }
}

impl Folio for PhysPage {
    fn pfn(&self) -> PFN {
        ZONE.get_pfn(self)
    }

    fn order(&self) -> u32 {
        self.order as u32
    }
}

impl FolioList for PageList {
    type Folio = PhysPage;

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn peek_head(&mut self) -> Option<&mut Self::Folio> {
        let front = self.0.front().clone_pointer()?;

        unsafe { UnsafeRef::into_raw(front).as_mut() }
    }

    fn pop_head(&mut self) -> Option<&'static mut Self::Folio> {
        let front = self.0.front_mut().remove()?;

        unsafe { UnsafeRef::into_raw(front).as_mut() }
    }

    fn push_tail(&mut self, page: &'static mut Self::Folio) {
        let page = unsafe { UnsafeRef::from_raw(page as *const _) };

        self.0.push_back(page);
    }

    fn remove(&mut self, page: &mut Self::Folio) {
        let mut cursor = unsafe { self.0.cursor_mut_from_ptr(page as *const _) };
        cursor.remove();
    }
}

impl FolioListSized for PageList {
    const NEW: Self = PageList(LinkedList::new(PagesAdapter::NEW));
}
