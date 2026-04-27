use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    interrupt::{interrupt::schedule_on_user_return, send_master_eoi, Registers},
    interrupt_entry,
    machine::{asm::enable_interrupts, TrapFrame},
    proc::{Channel, ProcessManager, ProcessState},
    serial::KResult,
    sync::IrqGuard,
    user::Userspace,
};

const HZ: usize = 60;

static JIFFIES: AtomicUsize = AtomicUsize::new(0);
static TIME: AtomicUsize = AtomicUsize::new(0);
static TIMEOUT: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn time_interrupt_body(_regs: *mut Registers, context: &mut TrapFrame) {
    clock(context);
    schedule_on_user_return(context);
}

fn clock(context: &mut TrapFrame) {
    let current_status = {
        let current = Userspace::get().proc();
        if context.is_user() {
            Userspace::get().utime += 1;
        } else {
            Userspace::get().stime += 1;
        }

        current.cpu = (current.cpu + 1).min(1024);

        current.stat
    };

    // fetch_add returns the old value, add 1 to get current jiffies.
    let jiffies = JIFFIES.fetch_add(1, Ordering::Release) + 1;

    if jiffies < HZ {
        send_master_eoi();
        return;
    }

    JIFFIES.store(0, Ordering::Release);
    let time = TIME.fetch_add(1, Ordering::AcqRel) + 1;

    #[cfg(feature = "debug_timer")]
    crate::println_debug!("Time={}", time);

    if time == TIMEOUT.load(Ordering::Acquire) {
        #[cfg(feature = "debug_timer")]
        crate::println_debug!("Waking up all sleepers");
        ProcessManager::get().wakeup_all((&TIMEOUT).channel_addr());
    }

    if current_status == ProcessState::SRUN && !context.is_user() {
        send_master_eoi();
        return;
    }

    enable_interrupts();
    send_master_eoi();

    ProcessManager::get().recalc_pri();

    if ProcessManager::get().run_in != 0 {
        ProcessManager::get().run_in = 0;
        ProcessManager::get().wakeup_runin();
    }

    // Don't preempt in kernel space.
    if !context.is_user() {
        return;
    }

    let current = Userspace::get().proc();
    if current.should_process() {
        current.process_signal(context);
    }
    current.set_pri();
}

pub fn set_time(value: usize) {
    TIME.store(value, Ordering::Release);
}

pub fn get_time() -> usize {
    TIME.load(Ordering::Acquire)
}

/// Set a timer that should wake us up at or earlier than `wake`.
///
/// # Note
/// Call this function with IRQ disabled, otherwise we may have lost wakeups.
///
/// # Returns
/// - `true` if the timer is set.
/// - `false` if the timer has already expired.
fn set_timer(wake: usize) -> bool {
    let now = get_time();
    let cur_timeout = TIMEOUT.load(Ordering::Acquire);

    if now >= wake {
        return false;
    }

    // Keep the timeout if it's earlier than requested
    if cur_timeout <= now || cur_timeout > wake {
        TIMEOUT.store(wake, Ordering::Release);
    }

    true
}

pub fn sleep_user_until(wake: usize, pri: u32) -> KResult<()> {
    let _irq = IrqGuard::disable_save();

    while set_timer(wake) {
        #[cfg(feature = "debug_timer")]
        crate::println_debug!("Sleeping until {}", wake);

        Userspace::get()
            .proc()
            .sleep_user((&TIMEOUT).channel_addr(), pri)?;
    }

    Ok(())
}

interrupt_entry!(TimeInterruptEntrance, time_interrupt_body);
