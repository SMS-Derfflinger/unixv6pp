mod context;
mod manager;
mod process;

pub use context::TaskContext;
pub use manager::ProcessManager;
pub use process::{KernelStack, Process, ProcessState, Text};

/// A channel that sleepers can subscribe to.
///
/// Used by sleep and wakeup family functions.
pub trait Channel: Sized {
    fn channel_addr(&self) -> usize;
}

impl<T> Channel for &T {
    #[inline(always)]
    fn channel_addr(&self) -> usize {
        *self as *const T as usize
    }
}

impl Channel for usize {
    #[inline(always)]
    fn channel_addr(&self) -> usize {
        *self
    }
}

pub const PINOD: i32 = -90;
pub const EXPRI: i32 = -1;
