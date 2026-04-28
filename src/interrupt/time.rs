use core::sync::atomic::{AtomicU64, Ordering};

use riscv::register::time;

use crate::constants::platform::CPU_FREQ_HZ;

pub const INTERRUPTS_PER_SECOND: usize = 10;
const TICKS_PER_INTERRUPT: u64 = (CPU_FREQ_HZ / INTERRUPTS_PER_SECOND) as u64;

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn get_time() -> u32 {
    (ticks() / INTERRUPTS_PER_SECOND as u64) as u32
}

fn read_time() -> u64 {
    time::read64()
}

fn set_timer_absolute(deadline: u64) {
    sbi::timer::set_timer(deadline).expect("failed to program SBI timer");
}

pub fn init_timer() {
    set_next_timer();
}

pub fn set_next_timer() {
    set_timer_absolute(read_time().wrapping_add(TICKS_PER_INTERRUPT));
}

pub fn handle_timer_interrupt() -> u64 {
    let tick = TICKS.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    set_next_timer();
    tick
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}
