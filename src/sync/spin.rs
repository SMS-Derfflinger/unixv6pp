use eonix_spin::{NoContext, Spin, SpinGuard};
use eonix_sync_base::Relax;

pub trait SpinExt<T, R> {
    fn lock(&self) -> SpinGuard<'_, T, NoContext, R>;
}

impl<T, R> SpinExt<T, R> for Spin<T, R>
where
    R: Relax,
{
    fn lock(&self) -> SpinGuard<'_, T, NoContext, R> {
        self.lock_ctx::<NoContext>()
    }
}
