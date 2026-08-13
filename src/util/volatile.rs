use core::ptr::{read_volatile, write_volatile};

#[macro_export]
macro_rules! volatile {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident {
            $($field_vis:vis $field:ident: $field_type:ty),*
            $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name {
            $($field: $crate::util::volatile::Volatile<$field_type>),*
        }
    };
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Volatile<T: Copy>(T);

impl<T: Copy> Volatile<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }
    
    pub fn get(&self) -> &T {
        &self.0
    }
    
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
    
    pub fn read(&self) -> T {
        unsafe { read_volatile(&self.0) }
    }
    
    pub fn write(&mut self, value: T) {
        unsafe { write_volatile(&mut self.0, value) }
    }
}

impl<T: Copy, const N: usize> Volatile<[T; N]> {
    pub fn read_at(&self, i: usize) -> T {
        unsafe { read_volatile(&self.0[i]) }
    }
    
    pub fn write_at(&mut self, i: usize, value: T) {
        unsafe { write_volatile(&mut self.0[i], value) }
    }
}
