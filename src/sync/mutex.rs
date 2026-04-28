use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{proc::ProcessManager, user::Userspace};

#[derive(Debug, Default)]
pub struct Mutex<T>
where
    T: ?Sized,
{
    locked: AtomicBool,
    priority: i16,
    value: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub const fn new(value: T, priority: i16) -> Self {
        Self {
            locked: AtomicBool::new(false),
            priority,
            value: UnsafeCell::new(value),
        }
    }
}

impl<T> Mutex<T>
where
    T: ?Sized,
{
    /// # Safety
    /// This function is unsafe because the caller MUST ensure that we've got the
    /// exclusive access before calling this function.
    unsafe fn get_lock(&self) -> MutexGuard<'_, T> {
        MutexGuard {
            lock: self,
            // SAFETY: We are holding the lock, so we can safely access the value.
            value: unsafe { &mut *self.value.get() },
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| unsafe { self.get_lock() })
    }

    fn try_lock_weak(&self) -> Option<MutexGuard<'_, T>> {
        self.locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| unsafe { self.get_lock() })
    }

    #[cold]
    fn lock_slow_path(&self) -> MutexGuard<'_, T> {
        let proc = Userspace::get().proc();

        loop {
            proc.set_kernel_sleep(&self.locked, self.priority as i32);
            if let Some(guard) = self.try_lock_weak() {
                proc.finish_kernel_sleep();
                return guard;
            }

            ProcessManager::get().switch();
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        if let Some(guard) = self.try_lock() {
            // Quick path
            guard
        } else {
            self.lock_slow_path()
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: The exclusive access to the lock is guaranteed by the borrow checker.
        unsafe { &mut *self.value.get() }
    }
}

// SAFETY: As long as the value protected by the lock is able to be shared between threads,
//         we can send the lock between threads.
unsafe impl<T> Send for Mutex<T> where T: ?Sized + Send {}

// SAFETY: `RwLock` can provide exclusive access to the value it protects, so it is safe to
//         implement `Sync` for it as long as the protected value is `Send`.
unsafe impl<T> Sync for Mutex<T> where T: ?Sized + Send {}

pub struct MutexGuard<'a, T>
where
    T: ?Sized,
{
    pub(super) lock: &'a Mutex<T>,
    pub(super) value: &'a mut T,
}

impl<T> Drop for MutexGuard<'_, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        let locked = self.lock.locked.swap(false, Ordering::Release);
        debug_assert!(
            locked,
            "MutexGuard::drop(): unlock() called on an unlocked mutex.",
        );
        ProcessManager::get().wakeup_all(&self.lock.locked);
    }
}

impl<T> Deref for MutexGuard<'_, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> DerefMut for MutexGuard<'_, T>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<T, U> AsRef<U> for MutexGuard<'_, T>
where
    T: ?Sized,
    U: ?Sized,
    <Self as Deref>::Target: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        self.deref().as_ref()
    }
}

impl<T, U> AsMut<U> for MutexGuard<'_, T>
where
    T: ?Sized + AsMut<U>,
    U: ?Sized,
    <Self as Deref>::Target: AsMut<U>,
{
    fn as_mut(&mut self) -> &mut U {
        self.deref_mut().as_mut()
    }
}
