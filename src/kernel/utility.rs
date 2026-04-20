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

#[repr(C)]
pub struct SystemTime {
    second: i32,
    minute: i32,
    hour: i32,
    day_of_month: i32,
    month: i32,
    year: i32,
    day_of_week: i32,
}

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
