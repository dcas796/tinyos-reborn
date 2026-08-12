use x86::dtables::lidt;
use crate::interrupt::pic::{clear_irq_mask, init_pic};
use crate::interrupt::table::{init_idt, IDTR};

pub mod entry;
pub mod stack_frame;
mod table;
pub mod pic;
mod wait;
pub mod irq_guard;

pub const IRQ_OFFSET: u8 = 0x20;

pub fn init_interrupts() {
    init_idt();
    unsafe {
        x86::irq::disable();
        lidt(&*IDTR);
        x86::irq::enable();
    }
    init_pic(IRQ_OFFSET);
    clear_irq_mask(0); // Unmask timer IRQ
    clear_irq_mask(1); // Unmask keyboard IRQ
}

pub use table::register_int;
pub use table::register_irq;

#[macro_export]
macro_rules! irq {
    (
        #[irq($irq:expr)]
        $(#[$attr:meta])*
        extern "x86-interrupt" fn $name:ident($($arg:tt)*) {
            $($body:tt)*
        }
    ) => {
        $(#[$attr])*
        extern "x86-interrupt" fn $name($($arg)*) {
            let _guard = $crate::interrupt::irq_guard::IrqGuard::new($irq);
            $($body)*
        }
    };
}
