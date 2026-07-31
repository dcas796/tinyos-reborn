use core::cell::RefCell;
use uart_16550::backend::PioBackend;
use uart_16550::{Config, Uart16550};
use crate::util::interrupt_lock::{InterruptLock, InterruptLockRef};

const SERIAL_PORT: u16 = 0x3f8;
static LOGGER: InterruptLock<RefCell<Option<Logger>>> = InterruptLock::new(RefCell::new(None));

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        if let Some(logger) = $crate::log::logger().borrow_mut().as_mut() {
            use core::fmt::Write;
            _ = core::write!(logger, $($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {{
        if let Some(logger) = $crate::log::logger().borrow_mut().as_mut() {
            use core::fmt::Write;
            _ = core::writeln!(logger, $($arg)*);
        }
    }};
}

pub fn logger<'a>() -> InterruptLockRef<'a, RefCell<Option<Logger>>> {
    LOGGER.get()
}

pub fn init_log() {
    fn _inner() -> Result<(), &'static str> {
        if LOGGER.get().borrow().is_some() { return Ok(()); }
        let mut port = unsafe { Uart16550::new_port(SERIAL_PORT) }
            .map_err(|_| "Could not open serial port")?;
        port
            .init(Config::default())
            .map_err(|_| "Could not initialize serial port")?;
        *LOGGER.get().borrow_mut() = Some(Logger::new(port));
        Ok(())
    }
    let _ = _inner(); /* Ignore the result. If in the future it is required for boot, just add `.unwrap()` */
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
