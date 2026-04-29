use core::sync::atomic::{AtomicUsize, Ordering};

use riscv::register::time;

use crate::{
    constants::platform::CPU_FREQ_HZ,
    interrupt::{context::TrapContext, schedule_on_user_return},
    machine::asm::enable_interrupts,
    proc::{Channel, ProcessManager, ProcessState},
    serial::KResult,
    sync::IrqGuard,
    user::Userspace,
};

pub const INTERRUPTS_PER_SECOND: usize = 60;
const TICKS_PER_INTERRUPT: usize = CPU_FREQ_HZ / INTERRUPTS_PER_SECOND;

static TICKS: AtomicUsize = AtomicUsize::new(0);
static TIME: AtomicUsize = AtomicUsize::new(0);
static TIMEOUT: AtomicUsize = AtomicUsize::new(0);

pub fn get_time() -> usize {
    TIME.load(Ordering::Acquire)
}

pub fn set_time(value: usize) {
    TIME.store(value, Ordering::Release);
}

fn set_timer_absolute(deadline: u64) {
    sbi::timer::set_timer(deadline).expect("failed to program SBI timer");
}

pub fn init_timer() {
    set_next_timer();
}

pub fn set_next_timer() {
    set_timer_absolute(time::read64().wrapping_add(TICKS_PER_INTERRUPT as u64));
}

pub fn handle_timer_interrupt(context: &mut TrapContext) {
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
    let ticks = TICKS.fetch_add(1, Ordering::Release) + 1;

    if ticks < TICKS_PER_INTERRUPT {
        set_next_timer();
        return;
    }

    TICKS.store(0, Ordering::Release);
    let time = TIME.fetch_add(1, Ordering::AcqRel) + 1;

    #[cfg(feature = "debug_timer")]
    crate::println_debug!("Time={}", time);

    if time == TIMEOUT.load(Ordering::Acquire) {
        #[cfg(feature = "debug_timer")]
        crate::println_debug!("Waking up all sleepers");
        ProcessManager::get().wakeup_all((&TIMEOUT).channel_addr());
    }

    if current_status == ProcessState::SRUN && !context.is_user() {
        set_next_timer();
        return;
    }

    enable_interrupts();
    set_next_timer();

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

    schedule_on_user_return(context);
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
