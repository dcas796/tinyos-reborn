#[derive(Debug, Clone)]
pub struct InterruptGuard;

impl InterruptGuard {
    pub fn new() -> Self {
        unsafe { x86::irq::disable(); }
        Self
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        unsafe { x86::irq::enable(); }
    }
}