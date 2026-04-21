use crate::{
    interrupt::{PtContext, PtRegs, interrupt::schedule_on_user_return, send_master_eoi},
    interrupt_entry,
    machine::asm::enable_interrupts,
    proc::{ProcessManager, ProcessState},
    user::Userspace,
};

const HZ: i32 = 60 * 2;

static mut LBOLT: i32 = 0;
static mut TIME: u32 = 0;
static mut TOUT: u32 = 0;

#[no_mangle]
pub extern "C" fn time_interrupt_body(_regs: *mut PtRegs, context: *mut PtContext) {
    clock(context);
    schedule_on_user_return(context);
}

fn clock(context: *mut PtContext) {
    let Some(context_ref) = (unsafe { context.as_ref() }) else {
        send_master_eoi();
        return;
    };

    let from_user_mode = context_ref.from_user_mode();
    let current_status = {
        let current = Userspace::get().proc();
        if from_user_mode {
            Userspace::get().utime += 1;
        } else {
            Userspace::get().stime += 1;
        }

        current.cpu = (current.cpu + 1).min(1024);

        current.stat
    };

    unsafe {
        LBOLT += 1;
        if LBOLT < HZ {
            send_master_eoi();
            return;
        }
    }

    if current_status == ProcessState::SRUN && !from_user_mode {
        send_master_eoi();
        return;
    }

    unsafe {
        LBOLT -= HZ;
        TIME += 1;
    }

    enable_interrupts();
    send_master_eoi();

    if get_time() == time_tout() {
        ProcessManager::get().wakeup_all(time_tout_address());
    }

    ProcessManager::get().recalc_pri();

    if ProcessManager::get().run_in != 0 {
        ProcessManager::get().run_in = 0;
        ProcessManager::get().wakeup_runin();
    }

    if from_user_mode {
        let current = Userspace::get().proc();
        if current.should_process() {
            current.process_signal(unsafe { &mut *(context as *mut _) });
        }
        current.set_pri();
    }
}

pub fn set_time(value: u32) {
    unsafe {
        TIME = value;
    }
}

#[no_mangle]
pub extern "C" fn get_time() -> u32 {
    unsafe { TIME }
}

#[no_mangle]
pub extern "C" fn time_set(value: u32) {
    set_time(value);
}

#[no_mangle]
pub extern "C" fn time_tout() -> u32 {
    unsafe { TOUT }
}

#[no_mangle]
pub extern "C" fn time_set_tout(value: u32) {
    unsafe {
        TOUT = value;
    }
}

#[no_mangle]
pub extern "C" fn time_tout_address() -> usize {
    core::ptr::addr_of!(TOUT) as usize
}

interrupt_entry!(TimeInterruptEntrance, time_interrupt_body);
