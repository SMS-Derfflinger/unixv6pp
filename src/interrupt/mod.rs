use riscv::register::sie;

use crate::{
    interrupt::handler::_trap_entry,
    machine::asm,
    proc::ProcessManager,
    serial,
};

pub mod context;
pub mod handler;
pub mod plic;
pub mod time;

pub fn init_trap() {
    asm::write_stvec(_trap_entry as *const () as usize);
    ProcessManager::get().bind_current_trap_context();
}

pub fn init_interrupt_controller() {
    plic::init();
    time::init_timer();
    serial::enable_rx_interrupt();

    unsafe {
        sie::set_stimer();
        sie::set_sext();
    }

    asm::enable_interrupts();
}
