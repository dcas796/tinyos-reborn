use crate::interrupt::pic;

#[derive(Debug, Clone)]
pub struct IrqGuard {
    irq: u8,
}

impl IrqGuard {
    pub fn new(irq: u8) -> Self {
        Self {
            irq
        }
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        pic::irq_end(self.irq);
    }
}