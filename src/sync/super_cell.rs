use core::cell::UnsafeCell;

pub struct SuperCell<T: ?Sized> {
    value: UnsafeCell<T>,
}

impl<T> SuperCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }
}

unsafe impl<T: ?Sized> Sync for SuperCell<T> {}

impl<T: ?Sized> SuperCell<T> {
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let value = unsafe { &*self.value.get() };
        f(value)
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let value = unsafe { &mut *self.value.get() };
        f(value)
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }
}
