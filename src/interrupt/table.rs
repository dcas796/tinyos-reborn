use x86::dtables::DescriptorTablePointer;
use crate::interrupt::stack_frame::InterruptStackFrame;
use crate::{logln, timer};
use crate::util::unsafe_wrappers::{UnsafeSyncSend};

macro_rules! table {
    ($(
        #[int($n:expr)]
        $(#[$attr:meta])*
        extern "x86-interrupt" fn $name:ident($($arg:tt)*) {
            $($body:tt)*
        }
    )*
    $(
        #[irq($qn:expr)]
        $(#[$qattr:meta])*
        extern "x86-interrupt" fn $qname:ident($($qarg:tt)*) {
            $($qbody:tt)*
        }
    )*
    $(
        default $dname:ident for $dn:expr
    )*) => {
        $(
            $(#[$attr])*
            extern "x86-interrupt" fn $name($($arg)*) {
                $($body)*
            }
        )*
        $(
            $(#[$qattr])*
            extern "x86-interrupt" fn $qname($($qarg)*) {
                use $crate::interrupt::irq_guard::IrqGuard;
                let _guard = IrqGuard::new($qn);
                $($qbody)*
            }
        )*
        $(
            extern "x86-interrupt" fn $dname(stack_frame: InterruptStackFrame) {
                use $crate::logln;
                logln!("Interrupt {:#02x} received: {stack_frame}", $dn);
            }
        )*

        lazy_static! {
            static ref IDT: [u64; 256] = {
                use $crate::interrupt::IRQ_OFFSET;
                use $crate::interrupt::entry::IdtEntry;
                use x86::segmentation::{SegmentSelector, SystemDescriptorTypes32};
                use x86::Ring;

                const CODE_DESCRIPTOR_INDEX: u16 = 1;

                fn set_table_entry(table: &mut [u64], n: usize, isr: *const ()) {
                    table[n as usize] = IdtEntry::new(
                        isr as u32,
                        SegmentSelector::new(CODE_DESCRIPTOR_INDEX, Ring::Ring0),
                        SystemDescriptorTypes32::InterruptGate32,
                        Ring::Ring0
                    ).into_u64();
                }

                let mut table = [0; 256];
                $(set_table_entry(&mut table, $dn as usize, $dname as *const ());)*
                $(set_table_entry(&mut table, $n as usize, $name as *const ());)*
                $(set_table_entry(&mut table, (IRQ_OFFSET + $qn) as usize, $qname as *const ());)*
                table
            };
        }
    };
}

table! {
    #[int(0x80)]
    extern "x86-interrupt" fn int_80(stack_frame: InterruptStackFrame) {
        logln!("Interrupt 0x80 (syscall) received: {stack_frame}");
    }

    #[irq(timer::PIT_IRQ)]
    extern "x86-interrupt" fn irq_0(_stack_frame: InterruptStackFrame) {
        timer::__interrupt();
    }

    default int_00 for 0x00
    default int_01 for 0x01
    default int_02 for 0x02
    default int_03 for 0x03
    default int_04 for 0x04
    default int_05 for 0x05
    default int_06 for 0x06
    default int_07 for 0x07
    default int_08 for 0x08
    default int_09 for 0x09
    default int_0a for 0x0a
    default int_0b for 0x0b
    default int_0c for 0x0c
    default int_0d for 0x0d
    default int_0e for 0x0e
    default int_0f for 0x0f
    default int_10 for 0x10
    default int_11 for 0x11
    default int_12 for 0x12
    default int_13 for 0x13
    default int_14 for 0x14
    default int_15 for 0x15
}

lazy_static! {
    pub static ref IDTR: UnsafeSyncSend<DescriptorTablePointer<u64>> =
        DescriptorTablePointer::new_from_slice(&*IDT).into();
}
