use core::{arch::asm, mem::MaybeUninit, ptr};

use crate::{
    interrupt::set_time,
    kernel::kernel::rust_kernel_initialize,
    machine::asm::enable_interrupts,
    println,
    proc::ProcessManager,
    vesa::{vesa_init, VbeModeInfo},
};

use super::{
    diagnose::_diagnose_trace_on,
    splash::splash,
    syscall::_lib_open,
    utility::{_make_kernel_time, SystemTime},
};

const KERNEL_SPACE_START_ADDRESS: usize = 0xc0000000;
const VESA_MODE_INFO_ADDR: usize = KERNEL_SPACE_START_ADDRESS + 0x7e00;
const VESA_SCREEN_VADDR: usize = KERNEL_SPACE_START_ADDRESS + 128 * 1024 * 1024;
const PAGE_DIRECTORY_BASE_ADDRESS: u32 = 0x200000;
const TTY_PATH: *const u8 = b"/dev/tty1\0".as_ptr();
const FREAD: u32 = 0x1;
const FWRITE: u32 = 0x2;

#[repr(C, packed)]
struct VbeModeInfoCompat {
    attributes: u16,
    window_a: u8,
    window_b: u8,
    granularity: u16,
    window_size: u16,
    segment_a: u16,
    segment_b: u16,
    win_func_ptr: u32,
    pitch: u16,
    width: u16,
    height: u16,
    w_char: u8,
    y_char: u8,
    planes: u8,
    bpp: u8,
    banks: u8,
    memory_model: u8,
    bank_size: u8,
    image_pages: u8,
    reserved0: u8,
    red_mask: u8,
    red_position: u8,
    green_mask: u8,
    green_position: u8,
    blue_mask: u8,
    blue_position: u8,
    reserved_mask: u8,
    reserved_position: u8,
    direct_color_attributes: u8,
    framebuffer: u32,
    off_screen_mem_off: u32,
    off_screen_mem_size: u16,
    reserved1: [u8; 206],
}

unsafe extern "C" {
    fn _init_vesa_memory_map(
        video_memory_address: usize,
        virtual_memory_address: usize,
        video_memory_size: usize,
    );
    fn _load_task_register();
    fn _cmos_read_time(time: *mut SystemTime);
    fn _cmos_read_byte_low() -> i32;
    fn _cmos_read_byte_high() -> i32;
    fn _init_user_page_table();
    fn clear_screan();

    fn FileSystem_load_super_block() -> bool;

    fn cpp_init_root_cdir();
    fn cpp_set_kernel_time(value: u32);

    fn runtime();
    fn ExecShell();
}

#[no_mangle]
pub extern "C" fn rust_kernel_next() {
    init_vesa();

    unsafe {
        _load_task_register();
    }

    init_kernel_time();
    read_memory_size();

    enable_interrupts();

    rust_kernel_initialize();
    ProcessManager::get().setup_proc_zero();

    load_file_system();
    println!("Unix V6++ FileSystem Loaded......OK");
    println!("test ");

    unsafe {
        cpp_init_root_cdir();
    }

    open_tty();
    _diagnose_trace_on();

    // splash();
    copy_runtime_to_userspace();

    let pid = ProcessManager::get().new_init_proc();
    if pid <= 0 {
        panic!("Failed to create init proc");
    }

    ProcessManager::schedule();
}

fn init_vesa() {
    let mode_info = VESA_MODE_INFO_ADDR as *const VbeModeInfoCompat;
    let framebuffer = unsafe { ptr::addr_of!((*mode_info).framebuffer).read_unaligned() };
    let pitch = unsafe { ptr::addr_of!((*mode_info).pitch).read_unaligned() };
    let height = unsafe { ptr::addr_of!((*mode_info).height).read_unaligned() };

    unsafe {
        _init_vesa_memory_map(
            framebuffer as usize,
            VESA_SCREEN_VADDR,
            pitch as usize * height as usize,
        );
    }
    vesa_init(mode_info.cast::<VbeModeInfo>());
}

fn init_kernel_time() {
    let mut time = MaybeUninit::<SystemTime>::uninit();
    unsafe {
        _cmos_read_time(time.as_mut_ptr());
    }
    set_time(_make_kernel_time(time.as_ptr()));
}

fn read_memory_size() {
    let low_mem = unsafe { _cmos_read_byte_low() };
    let high_mem = unsafe { _cmos_read_byte_high() };
    let _mem_size_kb = ((high_mem << 8) + low_mem) + 1024;
}

fn load_file_system() {
    let ok = unsafe { FileSystem_load_super_block() };
    if !ok {
        panic!("Load SuperBlock Error....!");
    }
}

fn open_tty() {
    let fd_tty = _lib_open(TTY_PATH, FREAD);
    if fd_tty != 0 {
        panic!("STDIN Error!");
    }

    let fd_tty = _lib_open(TTY_PATH, FWRITE);
    if fd_tty != 1 {
        panic!("STDOUT Error!");
    }
}

fn copy_runtime_to_userspace() {
    let runtime_addr = runtime as *const () as usize;
    let exec_shell_addr = ExecShell as *const () as usize;
    let runtime_src = runtime_addr as *const u8;
    let runtime_len = exec_shell_addr - runtime_addr;

    for offset in 0..runtime_len {
        unsafe {
            let byte = runtime_src.add(offset).read_volatile();
            (offset as *mut u8).write_volatile(byte);
        }
    }
}
