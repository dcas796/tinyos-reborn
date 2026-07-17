use core::cell::{RefCell, RefMut};
use uart_16550::backend::PioBackend;
use uart_16550::{Config, Uart16550};

struct UnsafeSync<T>(T);
unsafe impl<T> Sync for UnsafeSync<T> {}

const SERIAL_PORT: u16 = 0x3f8;
// SAFETY: single-threaded kernel, will never log in interrupts
static LOGGER: UnsafeSync<RefCell<Option<Logger>>> = UnsafeSync(RefCell::new(None));

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::log::logger_mut().as_mut() {
            use core::fmt::Write;
            _ = core::write!(logger, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::log::logger_mut().as_mut() {
            use core::fmt::Write;
            _ = core::writeln!(logger, $($arg)*);
        }
    };
}

pub fn logger_mut<'a>() -> RefMut<'a, Option<Logger>> {
    LOGGER.0.borrow_mut()
}

pub fn init_log() {
    if LOGGER.0.borrow().is_some() { return; }
    let mut port = unsafe { Uart16550::new_port(SERIAL_PORT) }
        .expect("Could not open serial port");
    port
        .init(Config::default())
        .expect("Could not initialize serial port");
    *LOGGER.0.borrow_mut() = Some(Logger::new(port));
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
