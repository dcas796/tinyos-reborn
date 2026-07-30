use core::cell::RefCell;
use crate::interrupt::interrupt_guard::InterruptGuard;
use crate::timer::command::{AccessMode, Channel, Command, CountingMode, OperatingMode};
use crate::timer::error::InvalidFrequencyError;
use crate::util::unsafe_wrappers::UnsafeSync;

mod command;
pub mod error;

const COMMAND: u16 = 0x43;
const SYSTEM_TIMER: u16 = 0x40;

pub fn set_timer_freq(freq: u32) -> Result<(), InvalidFrequencyError> {
    if freq > 1193182 || freq <= 18 {
        return Err(InvalidFrequencyError);
    }

    let divisor: u16 = (1193180 / freq) as u16;
    unsafe {
        x86::io::outb(COMMAND, Command {
            channel: Channel::Channel0,
            access_mode: AccessMode::LowHighByte,
            operating_mode: OperatingMode::Mode3,
            counting_mode: CountingMode::Binary,
        }.into_bits());
        x86::io::outb(SYSTEM_TIMER, (divisor & 0xFF) as u8);
        x86::io::outb(SYSTEM_TIMER, (divisor >> 8) as u8);
    }

    Ok(())
}

#[allow(clippy::type_complexity)]
static TIMER_HANDLER: UnsafeSync<RefCell<Option<fn()>>> = UnsafeSync::new(RefCell::new(None));
pub fn set_timer_handler(f: fn()) {
    let _guard = InterruptGuard::new();
    *TIMER_HANDLER.borrow_mut() = Some(f);
}

pub const PIT_IRQ: u8 = 0x00;
pub fn __interrupt() {
    if let Some(f) = &*TIMER_HANDLER.borrow() {
        f()
    }
}
