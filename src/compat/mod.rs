use core::{ffi::CStr, num::NonZero};

use alloc::boxed::Box;
use eonix_mm::address::{Addr, PAddr};
use kernel_macros::define_class_compat;

use crate::{dev::buffer::PhysicalBlock, mm::SWAPPER_AREAS, sync::SpinExt, user::Userspace};

pub fn compat_flush_page_directory() {
    unsafe {
        core::arch::asm!(
            "mov {}, %cr3",
            in (reg) 0x200000,
            options(att_syntax),
        );
    }
}

pub fn compat_user_pt() -> &'static mut [usize; 2048] {
    unsafe {
        &mut *(0xc0202000 as *mut [usize; 2048])
    }
}

pub fn compat_get_time() -> u32 {
    extern "C" {
        fn get_time() -> u32;
    }

    unsafe {
        get_time()
    }
}

define_class_compat! {impl Utils{
    pub fn get_path() -> *mut u8 {
        let dirp = Userspace::get().dirp;
        let cstr = unsafe { CStr::from_ptr(dirp as *const i8) };
        let bytes = cstr.to_bytes_with_nul();

        let boxed = Box::<[u8]>::new_uninit_slice(bytes.len() + size_of::<usize>());
        let mut boxed = unsafe { boxed.assume_init() };

        let head_ptr = boxed.as_mut_ptr() as *mut usize;
        Box::into_raw(boxed);

        let data_ptr = head_ptr.wrapping_add(1) as *mut u8;

        unsafe {
            head_ptr.write(bytes.len());
            data_ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
        }

        data_ptr
    }

    pub fn put_path(path: *mut u8) {
        let head_ptr = (path as *mut usize).wrapping_sub(1);

        unsafe {
            let alloc_len = *head_ptr + size_of::<usize>();
            Box::from_raw(core::ptr::slice_from_raw_parts_mut(head_ptr as *mut u8, alloc_len));
        }
    }
}}

pub fn compat_phys_copy(from: PAddr, to: PAddr, len: usize) {
    extern "C" {
        fn phys_copy(from: usize, to: usize, len: usize);
    }

    unsafe {
        phys_copy(from.addr(), to.addr(), len);
    }
}

const SECTOR_SIZE: usize = 512;

pub fn compat_swap_alloc(bytes: usize) -> PhysicalBlock {
    let sectors = (bytes + SECTOR_SIZE - 1) / SECTOR_SIZE;
    assert_ne!(sectors, 0);

    let block = SWAPPER_AREAS.lock()
        .alloc(NonZero::new(sectors).expect("Alloc 0 swap blocks"))
        .expect("Out of swap space");

    PhysicalBlock(block.get() as u32)
}

pub fn compat_swap_free(blkno: PhysicalBlock, bytes: usize) {
    let start_blk = NonZero::new(blkno.0 as usize)
        .expect("Free swap block 0");
    let sectors = NonZero::new((bytes + SECTOR_SIZE - 1) / SECTOR_SIZE)
        .expect("Free 0 swap blocks");

    SWAPPER_AREAS.lock().free(start_blk,sectors);
}

define_class_compat!{impl Utility {
    pub fn panic(msg: *const i8) -> ! {
        panic!("{:?}", unsafe { CStr::from_ptr(msg) })
    }
}}
