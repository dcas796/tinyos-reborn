use uart_16550::SerialPort;

const SERIAL_PORT: u16 = 0x3f8;
static mut LOGGER: Option<SerialPort> = None;

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

pub fn logger() -> Option<&'static mut SerialPort> {
    // SAFETY: yes
    unsafe { (&raw mut LOGGER).as_mut_unchecked() }.as_mut()
}

pub fn log_init() {
    unsafe {
        let mut port = SerialPort::new(SERIAL_PORT);
        port.init();
        LOGGER = Some(port);
    }
}
