#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate lazy_static;

use alloc::string::String;
use crate::sysinfo::{sysinfo_memregion_t, sysinfo_t, MemoryRegions, MemoryType};
use crate::vga::{init_vga, VgaColor};
use core::slice;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use x86::dtables::{sidt, DescriptorTablePointer};
use crate::interrupt::entry::IdtEntry;
use crate::interrupt::{init_interrupts, pic};
use crate::io::keyboard::{set_keyboard_handler, ScanCodeSet, KeyboardLayout, PhysicalKey};
use crate::kalloc::init_allocator;
use crate::log::init_log;
use crate::timer::{set_timer_freq, set_timer_handler, PIT_IRQ};
use crate::vga::VgaColor::*;

mod sysinfo;
mod vga;
mod log;
mod kalloc;
mod interrupt;
#[macro_use]
mod util;
mod timer;
mod io;

pub static PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub static PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _start(info_raw: *const sysinfo_t) -> ! {
    let info = unsafe { &*info_raw };

    /* Initialize modules */
    init_interrupts();
    init_log();
    let regions = MemoryRegions::from(info.mem_regions);
    init_allocator(&regions)
        .expect("Failed to initialize memory allocator");
    init_vga();

    /* Log useful info */
    logln!("{PACKAGE_NAME} {PACKAGE_VERSION}");
    logln!("System info: {info:#x?}");

    println!("{Magenta}{PACKAGE_NAME} {Gray}{PACKAGE_VERSION}{End}\n");
    println!("Boot drive: {Yellow}{:#x}{End}", info.boot_drive);

    print_mem_regions(info.mem_regions);

    do_heap_test();
    do_interrupt_test();
    do_timer_test();
    do_keyboard_test();

    halt();
}

fn print_mem_regions(mem_regions: *mut sysinfo_memregion_t) {
    let memory_regions = MemoryRegions::from(mem_regions);

    println!("Reported memory layout: ");
    for memory_region in memory_regions.iter() {
        let color = match memory_region.region_type {
            MemoryType::Usable => Green,
            MemoryType::ElfExecutable => LightBlue,
            _ => Red,
        };
        println!(
            "{color}{:#9x} - {:?} - {:#x}{End}",
            memory_region.base_addr,
            memory_region.region_type,
            memory_region.base_addr + memory_region.size - 1
        );
    }

    let largest_region = memory_regions
        .iter()
        .filter(|region| region.region_type == MemoryType::Usable)
        .max_by_key(|region| region.size)
        .expect("No usable memory regions found");

    println!("Largest region base address: {Green}{:#x}{End}", largest_region.base_addr);
    println!("Largest region size: {Green}{:.1} MB{End}", largest_region.size as f32 / 1_000_000.0);
}

fn do_heap_test() {
    let str = String::from("hallo");
    println!("String 1: {str}, {:?}", str.as_ptr());

    let str2 = String::from("h2llo");
    println!("String 2: {str2}, {:?}", str2.as_ptr());

    println!("String 1 (again): {str}");

    drop(str);

    let str3 = String::from("h3llo");
    println!("String 3: {str3}, {:?}", str3.as_ptr());
}

fn do_interrupt_test() {
    let idt = unsafe {
        let mut table = DescriptorTablePointer::<IdtEntry>::default();
        sidt(&mut table);
        slice::from_raw_parts(table.base, (table.limit as usize + 1) / size_of::<IdtEntry>())
    };
    logln!("IDT (0x80): {:#x?}", idt[0x80]);
    let gdt = unsafe {
        let mut table = DescriptorTablePointer::<u64>::default();
        x86::dtables::sgdt(&mut table);
        slice::from_raw_parts(table.base, (table.limit as usize + 1) / size_of::<u64>())
    };
    logln!("GDT: {:#x?}", gdt);
    unsafe {
        x86::int!(0x80);
    };
}

fn do_timer_test() {
    const TIMER_FREQ: u32 = 19;

    set_timer_handler(|| {
        static COUNT_TICKS: AtomicUsize = AtomicUsize::new(0);
        static COUNT_TIMES: AtomicUsize = AtomicUsize::new(0);

        if COUNT_TICKS.load(Ordering::Relaxed) == TIMER_FREQ as usize {
            COUNT_TICKS.store(0, Ordering::Relaxed);
            COUNT_TIMES.fetch_add(1, Ordering::Relaxed);
            print!(".");
            if COUNT_TIMES.load(Ordering::Relaxed) == 3 {
                println!();
                pic::set_irq_mask(PIT_IRQ);
            }
        } else {
            COUNT_TICKS.fetch_add(1, Ordering::Relaxed);
        }
    });

    if let Err(e) = set_timer_freq(TIMER_FREQ) {
        logln!("Failed timer test: {e:?}");
    }
}

fn do_keyboard_test() {
    set_keyboard_handler(|scan_code, meta| {
        static COLOR: AtomicU8 = AtomicU8::new(0x7);

        if let Some(physical_key) = scan_code.physical_key(ScanCodeSet::default()) {
            logln!("key: {physical_key:?}, is_down: {}", scan_code.is_down());
            if scan_code.is_down() {
                if let PhysicalKey::LeftGui | PhysicalKey::RightGui = physical_key {
                    COLOR.fetch_add(1, Ordering::Relaxed);
                    COLOR.fetch_and(0xf, Ordering::Relaxed);
                } else if let Some(c) = physical_key.as_char(KeyboardLayout::default(), meta) {
                    print!(
                        "{}{c}{End}",
                        VgaColor::try_from(COLOR.load(Ordering::Relaxed)).unwrap_or(End)
                    );
                }
            }
        } else {
            logln!("Unrecognized scan code: {scan_code}");
        }
    })
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
