use eonix_spin::{NoContext, Spin, SpinContext, SpinGuard};
use eonix_sync_base::SpinRelax;

use crate::machine::asm::disable_interrupts;
use riscv::register::sstatus::{self, Sstatus};

pub struct IrqContext(Sstatus);

pub type KernelSpinGuard<'a, T> = SpinGuard<'a, T, NoContext>;

impl SpinContext for IrqContext {
    fn save() -> Self {
        let flags = sstatus::read();
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
            sstatus::write(self.0);
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
