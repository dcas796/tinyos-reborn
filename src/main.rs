#![no_std]
#![no_main]

use core::arch::asm;
use core::ptr::{slice_from_raw_parts_mut, write_volatile};
use crate::sysinfo::sysinfo_t;

mod sysinfo;

#[unsafe(no_mangle)]
pub extern "C" fn _start(info: sysinfo_t) -> ! {
    unsafe {
        write_volatile(0xb8000 as *mut u16, (0x0fu16 << 8) | '#' as u16);
        write_volatile(0xb8002 as *mut u16, (0x0fu16 << 8) | (info.boot_drive - 2) as u16);

        loop {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}


