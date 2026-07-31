use x86::bits32::eflags::EFlags;

#[derive(Debug, Clone)]
pub struct InterruptGuard {
    were_interrupts_disabled: bool,
}

impl InterruptGuard {
    pub fn new() -> Self {
        let were_interrupts_disabled = !unsafe { x86::bits32::eflags::read() }
            .contains(EFlags::FLAGS_IF);
        if !were_interrupts_disabled {
            unsafe { x86::irq::disable() };
        }
        Self {
            were_interrupts_disabled
        }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if !self.were_interrupts_disabled {
            unsafe { x86::irq::enable() };
        }
    }
}
