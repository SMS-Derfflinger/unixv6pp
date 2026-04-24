use crate::{
    dev::{ata_driver::ATADriver, io_port::IOPort},
    interrupt::{PtContext, Registers, PIC_EOI, PIC_MASTER_IO_PORT_1},
    interrupt_entry,
    machine::asm::disable_interrupts,
    proc::ProcessManager,
    sync::IrqGuard,
    tty::keyboard::keyboard_handle_interrupt,
};

pub fn send_master_eoi() {
    unsafe {
        IOPort::out_byte(PIC_MASTER_IO_PORT_1, PIC_EOI);
    }
}

pub fn schedule_on_user_return(context: *mut PtContext) {
    let Some(context) = (unsafe { context.as_ref() }) else {
        return;
    };

    if !context.from_user_mode() {
        return;
    }

    loop {
        let ctx = IrqGuard::disable_save();
        disable_interrupts();

        if ProcessManager::get().runrun <= 0 {
            break;
        }

        ProcessManager::get().switch();
    }
}

#[no_mangle]
pub extern "C" fn disk_interrupt_body(_regs: *mut Registers, context: *mut PtContext) {
    ATADriver::ata_handler();
    schedule_on_user_return(context);
}

#[no_mangle]
pub extern "C" fn keyboard_interrupt_body(_regs: *mut Registers, context: *mut PtContext) {
    keyboard_handle_interrupt();
    send_master_eoi();
    schedule_on_user_return(context);
}

#[no_mangle]
pub extern "C" fn master_irq7(_regs: *mut Registers, _context: *mut PtContext) {
    crate::println_info!("IRQ7 from master 8295A");
    send_master_eoi();
}

interrupt_entry!(DiskInterruptEntrance, disk_interrupt_body);
interrupt_entry!(KeyboardInterruptEntrance, keyboard_interrupt_body);
interrupt_entry!(MasterIRQ7Entrance, master_irq7);
