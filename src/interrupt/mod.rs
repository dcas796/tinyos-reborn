use x86::dtables::lidt;
use crate::interrupt::pic::{clear_irq_mask, init_pic};
use crate::interrupt::table::IDTR;

pub mod entry;
mod stack_frame;
mod table;
pub mod pic;
mod wait;
pub mod interrupt_guard;
mod irq_guard;

pub const IRQ_OFFSET: u8 = 0x20;

pub fn init_interrupts() {
    unsafe {
        x86::irq::disable();
        lidt(&*IDTR);
        x86::irq::enable();
    }
    init_pic(IRQ_OFFSET);
    clear_irq_mask(0); // Unmask timer IRQ
}
