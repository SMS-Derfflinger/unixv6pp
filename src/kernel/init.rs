use core::{arch::naked_asm, mem::MaybeUninit, ptr, sync::atomic::compiler_fence};

use crate::{
    compat::compat_flush_page_directory,
    dev::{buffer::DevId, device_manager::ROOTDEV},
    fs::{global_file_system, InodeFlag, GLOBAL_INODE_TABLE},
    interrupt::set_time,
    kernel::kernel::rust_kernel_initialize,
    machine::{
        asm::enable_interrupts,
        chip::{
            cmos_read_byte_high, cmos_read_byte_low, cmos_read_time, init_peripherals, SystemTime,
        },
        enable_page_protection, init_gdt, init_idt, init_page_directory, init_user_page_table,
        load_gdt, load_idt,
    },
    mm::init_page_managers,
    println,
    proc::{KernelStack, ProcessManager},
    sync::SpinExt,
    user::Userspace,
    vesa::{vesa_clear, vesa_init, VbeModeInfo},
};

use super::{diagnose::_diagnose_trace_on, syscall::_lib_open, utility::_make_kernel_time};

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
    fn cpp_set_kernel_time(value: u32);
}

fn rust_kernel_next() {
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

    {
        let iref = GLOBAL_INODE_TABLE.lock().i_get(DevId(ROOTDEV), 1).unwrap();
        let mut inode = iref.lock();
        inode.i_flag.remove(InodeFlag::ILOCK);
        drop(inode);
        Userspace::get().cwd = Some(iref.into_inner());
    }

    open_tty();
    _diagnose_trace_on();

    // splash();

    init_user_page_table();
    compat_flush_page_directory();

    copy_runtime_to_userspace();

    vesa_clear(0);

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
    cmos_read_time(time.as_mut_ptr());
    set_time(_make_kernel_time(time.as_ptr()));
}

fn read_memory_size() {
    let low_mem = cmos_read_byte_low();
    let high_mem = cmos_read_byte_high();
    let _mem_size_kb = ((high_mem << 8) + low_mem) + 1024;
}

fn load_file_system() {
    let ok = global_file_system().load_super_block().is_ok();
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

#[unsafe(naked)]
unsafe extern "C" fn runtime() {
    naked_asm!(
        "mov %esp, %ebp",
        "call *%eax",   // entry
        "mov $1, %eax", // sys_exit
        "mov $0, %ebx", // $? = 0
        "int $0x80",
        options(att_syntax),
    )
}

fn copy_runtime_to_userspace() {
    let runtime_addr = runtime as *const u8;

    unsafe {
        (0x10 as *mut u8).copy_from_nonoverlapping(runtime_addr, 0x20);
    }
}

#[no_mangle]
pub extern "C" fn kernelBridge() -> ! {
    init_bss();

    // Don't move things across here
    compiler_fence(core::sync::atomic::Ordering::SeqCst);

    init_peripherals();

    init_gdt();
    load_gdt();

    init_idt();
    load_idt();

    init_page_directory();
    init_user_page_table();
    enable_page_protection();

    init_page_managers();
    let kstack = KernelStack::new();
    let top = kstack.top();
    core::mem::forget(kstack);

    unsafe {
        core::arch::asm!(
            "mov $0x10, %ax",
            "mov %ax, %ds",
            "mov %ax, %es",
            "mov %ax, %ss",
            "mov {top}, %ebp",
            "mov {top}, %esp",
            "ljmp $0x08, ${entry}",
            top = in(reg) top,
            entry = sym rust_kernel_next,
            options(att_syntax),
        );
    }

    unreachable!("main0() should never return");
}

fn init_bss() {
    extern "C" {
        fn __BSS_START__();
        fn __BSS_END__();
    }

    let bss_start = __BSS_START__ as usize;
    let bss_end = __BSS_END__ as usize;
    let bss_len = bss_end - bss_start;

    unsafe {
        core::slice::from_raw_parts_mut(bss_start as *mut u8, bss_len).fill(0);
    }
}
