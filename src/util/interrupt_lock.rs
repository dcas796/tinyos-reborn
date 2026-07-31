use crate::util::interrupt_guard::InterruptGuard;

pub struct InterruptLockRef<'a, T> {
    inner: &'a T,
    _guard: InterruptGuard,
}

impl<'a, T> InterruptLockRef<'a, T> {
    pub fn new(inner: &'a T) -> Self {
        Self {
            inner,
            _guard: InterruptGuard::new(),
        }
    }
}

impl<'a, T> core::ops::Deref for InterruptLockRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

pub struct InterruptLock<T> {
    inner: T,
}

impl<T> InterruptLock<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
        }
    }
    
    pub fn get(&self) -> InterruptLockRef<'_, T> {
        InterruptLockRef::new(&self.inner)
    }
}

unsafe impl<T> Sync for InterruptLock<T> {}
unsafe impl<T> Send for InterruptLock<T> {}
