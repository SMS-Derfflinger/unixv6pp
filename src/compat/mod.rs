#[cfg(target_arch = "x86")]
use crate::{dev::buffer::PhysicalBlock, mm::SWAPPER_AREAS, sync::SpinExt};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PhysicalBlock(pub u32);

pub fn compat_flush_page_directory() {
    unsafe {
        core::arch::asm!(
            "sfence.vma x0, x0",
            options(nostack, preserves_flags),
        );
    }
}

pub fn compat_user_pt() -> &'static mut [usize; 2048] {
    unsafe extern "C" {
        fn _user_page_table_array() -> *mut usize;
    }

    unsafe { &mut *_user_page_table_array().cast::<[usize; 2048]>() }
}

const SECTOR_SIZE: usize = 512;

#[cfg(target_arch = "x86")]
pub fn compat_swap_alloc(bytes: usize) -> PhysicalBlock {
    let sectors = (bytes + SECTOR_SIZE - 1) / SECTOR_SIZE;
    assert_ne!(sectors, 0);

    let block = SWAPPER_AREAS
        .lock()
        .alloc(NonZero::new(sectors).expect("Alloc 0 swap blocks"))
        .expect("Out of swap space");

    PhysicalBlock(block.get() as u32)
}

pub fn compat_swap_alloc(_bytes: usize) -> PhysicalBlock {
    panic!("swap is not wired up on riscv64 yet")
}

#[cfg(target_arch = "x86")]
pub fn compat_swap_free(blkno: PhysicalBlock, bytes: usize) {
    let start_blk = NonZero::new(blkno.0 as usize).expect("Free swap block 0");
    let sectors =
        NonZero::new((bytes + SECTOR_SIZE - 1) / SECTOR_SIZE).expect("Free 0 swap blocks");

    SWAPPER_AREAS.lock().free(start_blk, sectors);
}

pub fn compat_swap_free(_blkno: PhysicalBlock, _bytes: usize) {}
