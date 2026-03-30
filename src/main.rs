#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use crate::sysinfo::{sysinfo_memregion_t, sysinfo_t, MemoryRegions, MemoryType};
use crate::vga::Vga;
use core::fmt::Write;
use crate::kalloc::init_allocator;
use crate::log::init_log;
use crate::vga::VgaColor::*;

mod sysinfo;
mod vga;
mod log;
mod kalloc;

pub static PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub static PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _start(info_raw: *const sysinfo_t) -> ! {
    let info = unsafe { &*info_raw };

    init_modules(info);

    logln!("{PACKAGE_NAME} {PACKAGE_VERSION}");
    logln!("System info: {info:#x?}");

    let mut vga = Vga::default();
    vga.clear_screen();
    vga.set_foreground(White);
    vga.set_background(Black);

    writeln!(vga, "{Magenta}{PACKAGE_NAME} {Gray}{PACKAGE_VERSION}{End}\n").unwrap();
    writeln!(vga, "Boot drive: {Yellow}{:#x}{End}", info.boot_drive).unwrap();

    print_mem_regions(&mut vga, info.mem_regions);

    do_heap_test(&mut vga);

    halt();
}

fn init_modules(info: &sysinfo_t) {
    init_log();
    let memory_regions = MemoryRegions::from(info.mem_regions);
    init_allocator(&memory_regions).expect("Failed to initialize memory allocator");
}

fn print_mem_regions(vga: &mut Vga, mem_regions: *mut sysinfo_memregion_t) {
    let memory_regions = MemoryRegions::from(mem_regions);

    writeln!(vga, "Reported memory layout: ").unwrap();
    for memory_region in memory_regions.iter() {
        let color = match memory_region.region_type {
            MemoryType::Usable => Green,
            MemoryType::ElfExecutable => LightBlue,
            _ => Red,
        };
        writeln!(
            vga, "{color}{:#9x} - {:?} - {:#x}{End}",
            memory_region.base_addr,
            memory_region.region_type,
            memory_region.base_addr + memory_region.size - 1
        ).unwrap();
    }

    let largest_region = memory_regions
        .iter()
        .filter(|region| region.region_type == MemoryType::Usable)
        .max_by_key(|region| region.size)
        .expect("No usable memory regions found");

    writeln!(vga, "Largest region base address: {Green}{:#x}{End}", largest_region.base_addr).unwrap();
    writeln!(vga, "Largest region size: {Green}{:.1} MB{End}", largest_region.size as f32 / 1_000_000.0).unwrap();
}

fn do_heap_test(vga: &mut Vga) {
    let str = String::from("hallo");
    writeln!(vga, "String 1: {str}, {:?}", str.as_ptr()).unwrap();

    let str2 = String::from("h2llo");
    writeln!(vga, "String 2: {str2}, {:?}", str2.as_ptr()).unwrap();

    writeln!(vga, "String 1 (again): {str}").unwrap();

    drop(str);

    let str3 = String::from("h3llo");
    writeln!(vga, "String 3: {str3}, {:?}", str3.as_ptr()).unwrap();
}

pub fn halt() -> ! {
    loop {
        // SAFETY: I'm the kernel
        unsafe {
            x86::halt();
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    logln!("PANIC: {info}");
    halt();
}


