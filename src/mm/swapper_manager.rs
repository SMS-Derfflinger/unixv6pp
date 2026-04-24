use core::num::NonZero;

use eonix_spin::Spin;
use eonix_sync_base::LazyLock;

const SWAP_SECTORS: usize = 0x800;
const SWAP_SECTORS_OFF: usize = 0x7800;

pub static SWAPPER_AREAS: LazyLock<Spin<AreaList>> = LazyLock::new(|| {
    let mut areas = AreaList::new();
    areas.free(
        NonZero::new(SWAP_SECTORS_OFF).unwrap(),
        NonZero::new(SWAP_SECTORS).unwrap(),
    );

    Spin::new(areas)
});

#[derive(Clone, Copy)]
struct Area {
    addr: NonZero<usize>,
    size: NonZero<usize>,
}

pub struct AreaList {
    areas: [Option<Area>; Self::MAX],
}

impl Area {
    fn end(&self) -> NonZero<usize> {
        self.addr.checked_add(self.size.get()).unwrap()
    }
}

impl AreaList {
    const MAX: usize = 512;

    pub const fn new() -> Self {
        Self {
            areas: [const { None }; Self::MAX],
        }
    }

    fn remove(&mut self, idx: usize) {
        for i in idx..Self::MAX-1 {
            if self.areas[i+1].is_none() {
                self.areas[i] = None;
                break;
            }
            self.areas[i] = self.areas[i+1];
        }
    }

    fn pre_insert(&mut self, idx: usize) {
        let end = self.areas.iter()
            .position(|a| a.is_none()).expect("Overflow");

        for i in (idx+1..=end).rev() {
            self.areas[i] = self.areas[i-1];
        }

        self.areas[idx] = None;
    }

    pub fn alloc(&mut self, size: NonZero<usize>) -> Option<NonZero<usize>> {
        let found_idx = self.areas.iter_mut()
            .filter_map(Option::as_mut).position(|area| area.size >= size)?;
        let found = self.areas[found_idx].as_mut().unwrap();
        let addr = found.addr;

        if found.size > size {
            found.addr = found.addr.checked_add(size.get()).unwrap();
            found.size = NonZero::new(found.size.get() - size.get()).unwrap();
            return Some(addr);
        }

        // found.size == size
        self.remove(found_idx);

        Some(addr)
    }

    fn find_insert_pos(&self, addr: NonZero<usize>) -> usize {
        self.areas.iter()
            .position(|a| a.is_none() || a.unwrap().end() >= addr)
            .expect("Overflow")
    }

    fn try_merge_next(&mut self, pos: usize) {
        if pos == Self::MAX - 1 {
            return;
        }

        let (Some(cur), Some(next)) = (self.areas[pos], self.areas[pos + 1]) else {
            return;
        };

        if cur.end() != next.addr {
            return;
        }

        self.areas[pos] = Some(Area {
            addr: cur.addr,
            size: cur.size.checked_add(next.size.get()).unwrap(),
        });
        self.remove(pos + 1);
    }

    fn try_merge_prev(&mut self, pos: usize) {
        if pos == 0 {
            return;
        }

        let (Some(prev), Some(cur)) = (self.areas[pos - 1], self.areas[pos]) else {
            return;
        };

        if prev.end() != cur.addr {
            return;
        }

        self.areas[pos - 1] = Some(Area {
            addr: prev.addr,
            size: prev.size.checked_add(cur.size.get()).unwrap()
        });
        self.remove(pos);
    }

    pub fn free(&mut self, addr: NonZero<usize>, size: NonZero<usize>) {
        let pos = self.find_insert_pos(addr);

        let Some(area) = self.areas[pos].as_mut() else {
            self.areas[pos] = Some(Area { addr, size });
            return;
        };

        if area.end() == addr {
            area.size = area.size.checked_add(size.get()).unwrap();
        } else {
            self.pre_insert(pos);
            self.areas[pos] = Some(Area { addr, size });
        }

        self.try_merge_next(pos);
        self.try_merge_prev(pos);
    }
}
