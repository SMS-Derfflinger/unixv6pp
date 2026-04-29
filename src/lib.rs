#![no_std]

extern crate alloc;

mod dev;
mod fs;
mod interrupt;
#[cfg(target_arch = "x86")]
mod kernel;
mod loader;
pub mod machine;
pub mod mm;
pub mod proc;
pub mod tty;
mod user;
#[cfg(target_arch = "x86")]
mod vesa;

mod constants;
mod print;
mod serial;
pub mod sync;

use core::arch::naked_asm;
use core::panic::PanicInfo;

use crate::{
    dev::{buffer::DevId, buffer_manager::buffer_manager_initialize, device_manager::ROOTDEV},
    fs::{global_file_system, InodeFlag, GLOBAL_INODE_TABLE},
    proc::ProcessManager,
    sync::SpinExt,
    user::Userspace,
};

const TTY_PATH: *const u8 = b"/dev/tty1\0".as_ptr();
const FREAD: u32 = 0x1;
const FWRITE: u32 = 0x2;

pub trait Ext {
    fn as_buffer(&mut self) -> &mut [u8];
}

impl<T> Ext for T
where
    T: Copy,
{
    fn as_buffer(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

impl<T> Ext for [T]
where
    T: Copy,
{
    fn as_buffer(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                self.len() * core::mem::size_of::<T>(),
            )
        }
    }
}

#[unsafe(link_section = ".bootstrap.stack")]
static BOOT_STACK: [u8; 4096 * 4] = [0; 4096 * 4];

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".bootstrap.entry")]
unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "la   sp, {boot_stack}",
        "li   t0, {stack_size}",
        "add  sp, sp, t0",
        "la   t0, {rust_entry}",
        "jr   t0",
        boot_stack = sym BOOT_STACK,
        stack_size = const BOOT_STACK.len(),
        rust_entry = sym riscv64_rust_entry,
    )
}

#[unsafe(no_mangle)]
extern "C" fn riscv64_rust_entry(hart_id: usize, dtb_addr: usize) -> ! {
    clear_bss();
    machine::init_page_directory();
    machine::init_user_page_table();
    machine::enable_page_protection();
    serial::init_serial();
    println_info!("rust_kernel: entered riscv64 rust entry via OpenSBI");
    println_info!("paging enabled with global Sv39 page tables");
    println_info!("  hartid = {:#x}", hart_id);
    println_info!("  dtb    = {:#x}", dtb_addr);
    mm::init_page_managers();
    ProcessManager::get().setup_proc_zero();
    interrupt::init_trap();

    #[cfg(feature = "switchtest")]
    ProcessManager::get().run_kernel_switch_self_test();

    buffer_manager_initialize();
    interrupt::init_interrupt_controller();
    println_info!(
        "timer armed at {} Hz",
        interrupt::time::INTERRUPTS_PER_SECOND
    );

    load_file_system();
    println_info!("Unix V6++ FileSystem Loaded......OK");

    #[cfg(feature = "rvdebug")]
    interrupt_test();

    init_root_directory();
    open_console_tty();

    let pid = ProcessManager::get().new_init_proc();
    if pid == 0 {
        panic!("Failed to create init proc");
    }

    ProcessManager::schedule();
}

fn load_file_system() {
    let ok = global_file_system().load_super_block().is_ok();
    if !ok {
        panic!("Load SuperBlock Error....!");
    }
}

fn init_root_directory() {
    let iref = GLOBAL_INODE_TABLE
        .lock()
        .i_get(DevId(ROOTDEV), 1)
        .expect("failed to get root inode");
    let mut inode = iref.lock();
    inode.i_flag.remove(InodeFlag::ILOCK);
    drop(inode);
    Userspace::get().cwd = Some(iref.into_inner());
}

fn open_console_tty() {
    let fd_tty = kernel_open(TTY_PATH, FREAD);
    if fd_tty != 0 {
        panic!("STDIN Error!");
    }

    let fd_tty = kernel_open(TTY_PATH, FWRITE);
    if fd_tty != 1 {
        panic!("STDOUT Error!");
    }
}

fn kernel_open(pathname: *const u8, mode: u32) -> i32 {
    let user = Userspace::get();
    let saved_args = user.args;
    let saved_dirp = user.dirp;
    let saved_ar0 = user.ar0;
    let saved_error = user.error;
    let mut retval = usize::MAX;

    user.args[0] = pathname as usize;
    user.args[1] = mode as usize;
    user.dirp = pathname.cast_mut();
    user.ar0 = &raw mut retval;
    user.error = None;

    fs::syscall_open();

    let result = if user.error.is_some() {
        -1
    } else {
        retval as i32
    };

    user.args = saved_args;
    user.dirp = saved_dirp;
    user.ar0 = saved_ar0;
    user.error = saved_error;

    result
}

#[cfg(feature = "rvdebug")]
fn interrupt_test() {
    println_info!("before ebreak");
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack));
    }
    println_info!("after ebreak");
    println_info!("before illegal instruction");
    unsafe {
        core::arch::asm!(".word 0xffffffff", options(nomem, nostack));
    }
    println_info!("after illegal instruction");
}

fn clear_bss() {
    unsafe extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }

    let start = &raw mut __bss_start;
    let end = &raw mut __bss_end;
    let len = (end as usize).wrapping_sub(start as usize);

    unsafe {
        core::ptr::write_bytes(start, 0, len);
    }
}

#[cfg(target_arch = "x86")]
#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let msg = info.message();

    if let Some(msg) = msg.as_str() {
        println_fatal!("KERNEL PANIC: {}", msg);
    } else {
        println_fatal!("KERNEL PANIC: Unknown");
    }

    if let Some(loc) = info.location() {
        println_fatal!("Panicked at {}:{}", loc.file(), loc.line());
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    println_fatal!("rust_kernel: panic");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
