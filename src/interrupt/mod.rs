use riscv::register::sie;

use crate::{
    interrupt::{context::TrapContext, handler::_trap_entry},
    machine::asm,
    proc::ProcessManager,
    serial,
    sync::IrqGuard,
};

pub mod context;
pub mod handler;
pub mod plic;
pub mod system_call;
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

pub fn schedule_on_user_return(context: &mut TrapContext) {
    if !context.is_user() {
        return;
    }

    let _ctx = IrqGuard::disable_save();

    if ProcessManager::get().runrun <= 0 {
        return;
    }

    ProcessManager::get().switch();
}
