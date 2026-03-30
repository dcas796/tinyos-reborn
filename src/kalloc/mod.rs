mod kernel_allocator;

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
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
    kernel_allocator: UnsafeCell<Option<KernelMemoryAllocator>>,
}

impl GlobalAllocator {
    const fn empty() -> Self {
        Self { kernel_allocator: UnsafeCell::new(None) }
    }

    fn init(&self, regions: &MemoryRegions) -> Result<(), &'static str> {
        unsafe { *self.kernel_allocator.get() = Some(KernelMemoryAllocator::new(regions)?) };
        Ok(())
    }
}

macro_rules! call_allocator {
    ($a:expr,$n:ident,$($arg:tt)*) => {
        match unsafe { (&mut *$a.get()) } {
            Some(allocator) => allocator.$n($($arg)*),
            None => logln!("No allocator available"),
        }
    }
}

macro_rules! call_allocator_ret {
    ($a:expr,$n:ident,$($arg:tt)*) => {
        match unsafe { (&mut *$a.get()) } {
            Some(allocator) =>
                match allocator.$n($($arg)*) {
                    Some(ptr) => ptr.as_ptr(),
                    None => null_mut(),
                }
            None => {
                logln!("No allocator available");
                null_mut()
            }
        }
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        call_allocator_ret!(&self.kernel_allocator, alloc, layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let ptr_non_null = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => return,
        };
        call_allocator!(&self.kernel_allocator, dealloc, ptr_non_null, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        call_allocator_ret!(&self.kernel_allocator, alloc_zeroed, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let ptr_non_null = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => return null_mut(),
        };
        call_allocator_ret!(&self.kernel_allocator, realloc, ptr_non_null, layout, new_size)
    }
}

// Single-threaded kernel
unsafe impl Sync for GlobalAllocator {}
