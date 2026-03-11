#![no_std]
#![no_main]

use core::arch::asm;
use crate::sysinfo::sysinfo_t;
use crate::vga::Vga;
use core::fmt::Write;
use crate::log::log_init;

mod sysinfo;
mod vga;
mod log;

pub static PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub static PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[unsafe(no_mangle)]
pub extern "C" fn _start(info: sysinfo_t) -> ! {
    log_init();

    logln!("{PACKAGE_NAME} {PACKAGE_VERSION}");
    logln!("System info: {info:#x?}");

    let mut vga = Vga::default();
    vga.clear_screen();

    writeln!(vga, "{PACKAGE_NAME} {PACKAGE_VERSION}").unwrap();
    writeln!(vga, "System info: {info:#x?}").unwrap();

    halt();
}

pub fn halt() -> ! {
    loop {
        // SAFETY: duh
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    logln!("PANIC: {info}");
    halt();
}


