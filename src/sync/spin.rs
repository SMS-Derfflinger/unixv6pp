use eonix_spin::{NoContext, Spin, SpinContext, SpinGuard};
use eonix_sync_base::SpinRelax;

use crate::machine::asm::disable_interrupts;

pub struct IrqContext(u32);

pub type KernelSpinGuard<'a, T> = SpinGuard<'a, T, NoContext>;

impl SpinContext for IrqContext {
    fn save() -> Self {
        let flags;
        unsafe {
            core::arch::asm!(
                "pushf",
                "pop {}",
                out(reg) flags,
                options(att_syntax),
            );
        }

        disable_interrupts();
        Self(flags)
    }

    fn restore(mut self) {
        self._restore();
    }
}

impl IrqContext {
    fn _restore(&mut self) {
        unsafe {
            core::arch::asm!(
                "push {}",
                "popf",
                in(reg) self.0,
                options(att_syntax),
            );
        }
    }
}

pub trait SpinExt<T> {
    fn lock(&self) -> KernelSpinGuard<'_, T>;
}

impl<T> SpinExt<T> for Spin<T, SpinRelax> {
    fn lock(&self) -> KernelSpinGuard<'_, T> {
        self.lock_ctx::<NoContext>()
    }
}

pub struct IrqGuard(IrqContext);

impl IrqGuard {
    pub fn disable_save() -> Self {
        Self(IrqContext::save())
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        self.0._restore();
    }
}
