use eonix_mm::address::{Addr, PAddr};
use eonix_mm::paging::PFN;

use crate::compat::compat_flush_page_directory;
use crate::machine::chip::SystemTime;
use crate::machine::{global_user_page_table, kernel_page_table_mut, EntryFlags};
use crate::serial::KResult;
use crate::sync::IrqGuard;
use crate::user::Userspace;

const SECONDS_IN_MINUTE: u32 = 60;
const SECONDS_IN_HOUR: u32 = 3600;
const SECONDS_IN_DAY: u32 = 86400;
const DAYS_BEFORE_MONTH: [u32; 13] = [
    u32::MAX,
    0,
    31,
    59,
    90,
    120,
    151,
    181,
    212,
    243,
    273,
    304,
    334,
];

#[no_mangle]
pub extern "C" fn _mem_copy(src: usize, dst: usize, count: u32) {
    let src = src as *const u8;
    let dst = dst as *mut u8;

    for offset in 0..count as usize {
        unsafe {
            dst.add(offset).write(src.add(offset).read());
        }
    }
}

#[no_mangle]
pub extern "C" fn _calculate_page_need(memory_need: u32, page_size: u32) -> i32 {
    let mut page_required = memory_need / page_size;
    if memory_need % page_size != 0 {
        page_required += 1;
    }
    page_required as i32
}

#[no_mangle]
pub extern "C" fn _get_major(dev: i16) -> i16 {
    dev >> 8
}

#[no_mangle]
pub extern "C" fn _get_minor(dev: i16) -> i16 {
    dev & 0x00ff
}

#[no_mangle]
pub extern "C" fn _set_major(dev: i16, value: i16) -> i16 {
    (dev & 0x00ff) | (value << 8)
}

#[no_mangle]
pub extern "C" fn _set_minor(dev: i16, value: i16) -> i16 {
    (dev & !0x00ff) | (value & 0x00ff)
}

#[no_mangle]
pub extern "C" fn _dword_copy(src: *const i32, dst: *mut i32, count: i32) {
    for offset in 0..count.max(0) as usize {
        unsafe {
            dst.add(offset).write(src.add(offset).read());
        }
    }
}

#[no_mangle]
pub extern "C" fn _min(a: i32, b: i32) -> i32 {
    if a < b {
        a
    } else {
        b
    }
}

#[no_mangle]
pub extern "C" fn _max(a: i32, b: i32) -> i32 {
    if a > b {
        a
    } else {
        b
    }
}

#[no_mangle]
pub extern "C" fn _bcd_to_binary(value: i32) -> i32 {
    ((value >> 4) * 10) + (value & 0x0f)
}

#[no_mangle]
pub extern "C" fn _io_move(src: *const u8, dst: *mut u8, count: i32) {
    for offset in 0..count.max(0) as usize {
        unsafe {
            dst.add(offset).write(src.add(offset).read());
        }
    }
}

#[no_mangle]
pub extern "C" fn _make_kernel_time(time: *const SystemTime) -> u32 {
    let time = unsafe { time.as_ref().expect("_make_kernel_time null time") };
    let current_year = 2000 + time.year;

    let mut seconds = time.second as u32;
    seconds += time.minute as u32 * SECONDS_IN_MINUTE;
    seconds += time.hour as u32 * SECONDS_IN_HOUR;

    let mut days = (time.day_of_month - 1) as u32;
    days += DAYS_BEFORE_MONTH[time.month as usize];
    if _is_leap_year(current_year) && time.month >= 3 {
        days += 1;
    }

    for year in 1970..current_year {
        days += _days_in_year(year);
    }

    seconds + days * SECONDS_IN_DAY
}

#[no_mangle]
pub extern "C" fn _is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[no_mangle]
pub extern "C" fn _days_in_year(year: i32) -> u32 {
    if _is_leap_year(year) {
        366
    } else {
        365
    }
}

/// 对应 C++ 的 Utility::CopySeg
/// 通过内核页表中借用的两个 PTE（索引 256 和 257），
/// 将 src 物理地址处的一个字节复制到 dst 物理地址处。
fn copy_seg(src: usize, dst: usize) {
    const BORROWED_PTE: usize = 256;
    const KERNEL_SPACE_START: usize = 0xC0000000;
    const PAGE_SIZE: usize = 0x1000;

    let kernel_pt = kernel_page_table_mut();

    // 保存原页表项
    let ori_entry1 = kernel_pt[BORROWED_PTE].get();
    let ori_entry2 = kernel_pt[BORROWED_PTE + 1].get();

    // 将 src 和 dst 所在物理页映射到借用的 PTE
    let flags = EntryFlags::PRESENT | EntryFlags::WRITE;
    kernel_pt[BORROWED_PTE].set(Some(PFN::from_val(src / PAGE_SIZE)), flags);
    kernel_pt[BORROWED_PTE + 1].set(Some(PFN::from_val(dst / PAGE_SIZE)), flags);

    let addr_src = (KERNEL_SPACE_START + BORROWED_PTE * PAGE_SIZE + src % PAGE_SIZE) as *const u8;
    let addr_dst =
        (KERNEL_SPACE_START + (BORROWED_PTE + 1) * PAGE_SIZE + dst % PAGE_SIZE) as *mut u8;

    // 刷新页表缓存
    compat_flush_page_directory();

    unsafe {
        addr_dst.write_volatile(addr_src.read_volatile());
    }

    // 恢复原页表映射
    kernel_pt[BORROWED_PTE].set(Some(ori_entry1.0), ori_entry1.1);
    kernel_pt[BORROWED_PTE + 1].set(Some(ori_entry2.0), ori_entry2.1);
    compat_flush_page_directory();
}

/// 对应 C++ 的 Utility::CopySeg2
/// 通过用户页表的前两个 PTE（索引 0 和 1），
/// 将 src 物理地址处的一个字节复制到 dst 物理地址处。
fn copy_seg2(src: usize, dst: usize) {
    const PAGE_SIZE: usize = 0x1000;

    let user_pt = global_user_page_table();

    // 保存原页表项（用户页表第一张的第 0 项和第 1 项）
    let ori_entry1 = user_pt[0][0].get();
    let ori_entry2 = user_pt[0][1].get();

    // 将 src 和 dst 所在物理页映射到用户页表前两项
    let flags = EntryFlags::PRESENT | EntryFlags::WRITE | EntryFlags::USER;
    user_pt[0][0].set(Some(PFN::from_val(src / PAGE_SIZE)), flags);
    user_pt[0][1].set(Some(PFN::from_val(dst / PAGE_SIZE)), flags);

    let addr_src = (src % PAGE_SIZE) as *const u8;
    let addr_dst = (PAGE_SIZE + dst % PAGE_SIZE) as *mut u8;

    // 刷新页表缓存
    compat_flush_page_directory();

    unsafe {
        addr_dst.write_volatile(addr_src.read_volatile());
    }

    // 恢复原页表映射
    user_pt[0][0].set(Some(ori_entry1.0), ori_entry1.1);
    user_pt[0][1].set(Some(ori_entry2.0), ori_entry2.1);
    compat_flush_page_directory();
}

/// 对应 C++ 的 phys_copy
/// 逐字节将物理地址 from 处的 len 个字节复制到物理地址 to 处。
pub fn phys_copy(from: PAddr, to: PAddr, len: usize) {
    let _ctx = IrqGuard::disable_save();
    for i in 0..len {
        copy_seg(from.addr() + i, to.addr() + i);
    }
}

pub trait KResultExt {
    fn pass_to_user(self);
}

pub trait NativeWord {
    fn into_word(self) -> usize;
}

impl NativeWord for u32 {
    fn into_word(self) -> usize {
        self as usize
    }
}

impl NativeWord for usize {
    fn into_word(self) -> usize {
        self
    }
}

impl NativeWord for () {
    fn into_word(self) -> usize {
        0
    }
}

impl<T: NativeWord> KResultExt for KResult<T> {
    fn pass_to_user(self) {
        match self {
            Ok(retval) => Userspace::get().set_user_retval(retval.into_word() as u32),
            Err(err) => Userspace::get().set_error(err),
        }
    }
}
