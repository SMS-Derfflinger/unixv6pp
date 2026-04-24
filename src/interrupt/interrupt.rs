use crate::{
    dev::{ata_driver::ATADriver, io_port::IOPort},
    interrupt::{Registers, PIC_EOI, PIC_MASTER_IO_PORT_1},
    interrupt_entry,
    machine::TrapFrame,
    proc::ProcessManager,
    sync::IrqGuard,
    tty::keyboard::keyboard_handle_interrupt,
};

pub fn send_master_eoi() {
    unsafe {
        IOPort::out_byte(PIC_MASTER_IO_PORT_1, PIC_EOI);
    }
}

pub fn schedule_on_user_return(context: &mut TrapFrame) {
    if !context.is_user() {
        return;
    }

    let _ctx = IrqGuard::disable_save();

    if ProcessManager::get().runrun <= 0 {
        return;
    }

    ProcessManager::get().switch();
}

#[no_mangle]
pub extern "C" fn disk_interrupt_body(_regs: *mut Registers, context: &mut TrapFrame) {
    ATADriver::ata_handler();
    schedule_on_user_return(context);
}

#[no_mangle]
pub extern "C" fn keyboard_interrupt_body(_regs: *mut Registers, context: &mut TrapFrame) {
    keyboard_handle_interrupt();
    send_master_eoi();
    schedule_on_user_return(context);
}

#[no_mangle]
pub extern "C" fn master_irq7(_regs: *mut Registers, _context: &mut TrapFrame) {
    crate::println_info!("IRQ7 from master 8295A");
    send_master_eoi();
}

interrupt_entry!(DiskInterruptEntrance, disk_interrupt_body);
interrupt_entry!(KeyboardInterruptEntrance, keyboard_interrupt_body);
interrupt_entry!(MasterIRQ7Entrance, master_irq7);
