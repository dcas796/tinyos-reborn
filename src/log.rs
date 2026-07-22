use core::cell::{RefCell, RefMut};
use uart_16550::backend::PioBackend;
use uart_16550::{Config, Uart16550};
use crate::util::interrupt_guard::InterruptGuard;
use crate::util::unsafe_wrappers::UnsafeSync;

const SERIAL_PORT: u16 = 0x3f8;
// SAFETY: single-threaded kernel, will never log in interrupts
static LOGGER: UnsafeSync<RefCell<Option<Logger>>> = UnsafeSync::new(RefCell::new(None));

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use $crate::util::interrupt_guard::InterruptGuard;
        let _guard = InterruptGuard::new();
        if let Some(logger) = $crate::log::logger_mut().as_mut() {
            use core::fmt::Write;
            _ = core::write!(logger, $($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {{
        use $crate::util::interrupt_guard::InterruptGuard;
        let _guard = InterruptGuard::new();
        if let Some(logger) = $crate::log::logger_mut().as_mut() {
            use core::fmt::Write;
            _ = core::writeln!(logger, $($arg)*);
        }
    }};
}

pub fn logger_mut<'a>() -> RefMut<'a, Option<Logger>> {
    LOGGER.borrow_mut()
}

pub fn init_log() {
    if LOGGER.borrow().is_some() { return; }
    let mut port = unsafe { Uart16550::new_port(SERIAL_PORT) }
        .expect("Could not open serial port");
    port
        .init(Config::default())
        .expect("Could not initialize serial port");
    {
        let _guard = InterruptGuard::new();
        *LOGGER.borrow_mut() = Some(Logger::new(port));
    };
}

pub struct Logger {
    serial: Uart16550<PioBackend>,
}

impl Logger {
    fn new(serial: Uart16550<PioBackend>) -> Logger {
        Logger { serial }
    }
}

impl core::fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.serial.send_bytes_exact(s.as_bytes());
        Ok(())
    }
}
