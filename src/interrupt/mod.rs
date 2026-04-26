use riscv::register::sie;

use crate::{
    interrupt::{context::TrapContext, handler::_trap_entry},
    machine::asm,
};

pub mod context;
pub mod handler;
pub mod time;

static mut BOOT_TRAP_CONTEXT: TrapContext = TrapContext::new();

pub fn init_trap() {
    let trap_context = &raw mut BOOT_TRAP_CONTEXT as *mut TrapContext as usize;
    asm::write_sscratch(trap_context);
    asm::write_stvec(_trap_entry as *const () as usize);
}

pub fn init_interrupt_controller() {
    time::init_timer();

    unsafe {
        sie::set_stimer();
    }

    asm::enable_interrupts();
}
