use eonix_spin::{Spin, SpinContext, SpinGuard};
use eonix_sync_base::SpinRelax;

use crate::machine::asm::{disable_interrupts, enable_interrupts};

pub struct IrqContext;

pub type KernelSpinGuard<'a, T> = SpinGuard<'a, T, IrqContext>;

impl SpinContext for IrqContext {
    fn save() -> Self {
        disable_interrupts();
        Self
    }

    fn restore(self) {
        enable_interrupts();
    }
}

pub trait SpinExt<T> {
    fn lock(&self) -> KernelSpinGuard<'_, T>;
}

impl<T> SpinExt<T> for Spin<T, SpinRelax> {
    fn lock(&self) -> KernelSpinGuard<'_, T> {
        self.lock_ctx::<IrqContext>()
    }
}
