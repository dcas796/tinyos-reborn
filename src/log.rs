use uart_16550::backend::PioBackend;
use uart_16550::{Config, Uart16550};

const SERIAL_PORT: u16 = 0x3f8;
static mut LOGGER: Option<Logger> = None;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::log::logger() {
            use core::fmt::Write;
            _ = core::write!(logger, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::log::logger() {
            use core::fmt::Write;
            _ = core::writeln!(logger, $($arg)*);
        }
    };
}

pub fn logger() -> Option<&'static mut Logger> {
    // SAFETY: yes
    unsafe { (&raw mut LOGGER).as_mut_unchecked() }.as_mut()
}

pub fn init_log() {
    unsafe {
        let mut port = Uart16550::new_port(SERIAL_PORT)
            .expect("Could not open serial port");
        port
            .init(Config::default())
            .expect("Could not initialize serial port");
        LOGGER = Some(port.into());
    }
}

pub struct Logger {
    serial: Uart16550<PioBackend>,
}

impl From<Uart16550<PioBackend>> for Logger {
    fn from(serial: Uart16550<PioBackend>) -> Self {
        Self { serial }
    }
}

impl core::fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.serial.send_bytes_exact(s.as_bytes());
        Ok(())
    }
}
