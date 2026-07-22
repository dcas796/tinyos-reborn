use core::fmt::{Display, Formatter};
use x86::bits32::eflags::EFlags;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct InterruptStackFrame {
    pub ip: u32,
    pub cs: u32,
    pub flags: u32,
}

impl Display for InterruptStackFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "InterruptStackFrame {{
    ip: {:#x},
    cs: {:#x},
    flags: {:?} ({:#x}),
}}", self.ip, self.cs, EFlags::from_bits_truncate(self.flags), self.flags)?;
        Ok(())
    }
}
