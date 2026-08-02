use alloc::boxed::Box;

pub unsafe fn boxed_slice_from_nonaligned_ptr<T>(ptr: *const u8, len: usize) -> Box<[T]> {
    let mut slice = Box::<[T]>::new_uninit_slice(len);
    let slice_ptr = slice.as_mut_ptr() as *mut u8;
    unsafe { 
        slice_ptr.copy_from(ptr, len * size_of::<T>());
        slice.assume_init()
    }
}
