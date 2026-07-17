mod kernel_allocator;

use core::alloc::{GlobalAlloc, Layout};
use core::cell::OnceCell;
use core::ptr::{null_mut, NonNull};
use crate::kalloc::kernel_allocator::KernelMemoryAllocator;
use crate::logln;
use crate::sysinfo::MemoryRegions;

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::empty();



pub fn init_allocator(regions: &MemoryRegions) -> Result<(), &'static str> {
    ALLOCATOR.init(regions)
}

struct GlobalAllocator {
    kernel_allocator: OnceCell<KernelMemoryAllocator>,
}

struct InterruptGuard;

// TODO: Make this not crash QEMU
impl InterruptGuard {
    fn new() -> Self {
        // unsafe { x86::irq::disable(); }
        Self
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        // unsafe { x86::irq::enable(); }
    }
}

impl GlobalAllocator {
    const fn empty() -> Self {
        Self {
            kernel_allocator: OnceCell::new()
        }
    }

    fn init(&self, regions: &MemoryRegions) -> Result<(), &'static str> {
        let allocator = KernelMemoryAllocator::new(regions)?;
        self.kernel_allocator
            .set(allocator)
            .map_err(|_| "Allocator already initialized")
    }
}

macro_rules! call_allocator {
    ($a:expr,$n:ident,$($arg:tt)*) => {{
        let _guard = InterruptGuard::new();
        match $a.get() {
            Some(allocator) => allocator.$n($($arg)*),
            None => logln!("No allocator available"),
        }
    }}
}

macro_rules! call_allocator_ret {
    ($a:expr,$n:ident,$($arg:tt)*) => {{
        let _guard = InterruptGuard::new();
        match $a.get() {
            Some(allocator) =>
                match allocator.$n($($arg)*) {
                    Some(ptr) => ptr.as_ptr(),
                    None => null_mut(),
                }
            None => {
                logln!("No allocator available");
                null_mut()
            }
        }}
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        call_allocator_ret!(self.kernel_allocator, alloc, layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let ptr_non_null = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => return,
        };
        call_allocator!(self.kernel_allocator, dealloc, ptr_non_null, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        call_allocator_ret!(self.kernel_allocator, alloc_zeroed, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let ptr_non_null = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => return null_mut(),
        };
        call_allocator_ret!(self.kernel_allocator, realloc, ptr_non_null, layout, new_size)
    }
}

// SAFETY: every operation has an InterruptGuard, so that an allocation inside an interrupt inside
//   an outer allocation never happens.
unsafe impl Sync for GlobalAllocator {}
