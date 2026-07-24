use x86::dtables::lidt;
use crate::interrupt::table::IDTR;

pub mod entry;
mod stack_frame;
mod table;

pub fn init_interrupts() {
    unsafe {
        x86::irq::disable();
        lidt(&*IDTR);
        x86::irq::enable();
    }
}
