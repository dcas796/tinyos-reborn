use x86::dtables::{lidt, DescriptorTablePointer};
use x86::Ring;
use x86::segmentation::{SegmentSelector, SystemDescriptorTypes32};
use crate::interrupt::entry::IdtEntry;
use crate::interrupt::stack_frame::InterruptStackFrame;
use crate::logln;
use crate::util::unsafe_wrappers::UnsafeSyncSend;

pub mod entry;
mod stack_frame;

const CODE_DESCRIPTOR_INDEX: u16 = 1;

lazy_static! {
    static ref IDT: [u64; 256] = [IdtEntry::new(
        interrupt_handler as *const () as u32,
        SegmentSelector::new(CODE_DESCRIPTOR_INDEX, Ring::Ring0),
        SystemDescriptorTypes32::InterruptGate32,
        Ring::Ring0
    ).into_u64(); 256];

    static ref IDTR: UnsafeSyncSend<DescriptorTablePointer<u64>> =
        DescriptorTablePointer::new_from_slice(&*IDT).into();
}

extern "x86-interrupt" fn interrupt_handler(stack_frame: InterruptStackFrame) {
    logln!("Interrupt received: {stack_frame}");
}

pub fn init_interrupts() {
    unsafe {
        x86::irq::disable();
        lidt(&*IDTR);
        x86::irq::enable();
    }
}
