use x86::bits32::eflags::EFlags;

#[macro_export]
macro_rules! save_registers {
    ($stack_frame:expr) => {{
        ::core::arch::asm!(
            "push esp",
            "push ebp",
            "push edi",
            "push esi",
            "push edx",
            "push ecx",
            "push ebx",
            "push eax",
        );
        let mut stack_addr: u32;
        ::core::arch::asm!(
            "mov {}, esp",
            out(reg) stack_addr,
        );
        let stack_regs = ::core::slice::from_raw_parts(
            stack_addr as *const u32,
            8,
        );
        let stack_frame: $crate::interrupt::stack_frame::InterruptStackFrame = $stack_frame;
        $crate::interrupt::regs::Registers {
            ip: stack_frame.ip,
            cs: stack_frame.cs,
            flags: ::x86::bits32::eflags::EFlags::from_bits_truncate(stack_frame.flags),
            eax: stack_regs[0],
            ebx: stack_regs[1],
            ecx: stack_regs[2],
            edx: stack_regs[3],
            esi: stack_regs[4],
            edi: stack_regs[5],
            ebp: stack_regs[6],
            esp: stack_regs[7],
        }
    }};
}

#[derive(Debug, Copy, Clone)]
pub struct Registers {
    pub ip: u32,
    pub cs: u32,
    pub flags: EFlags,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
}
