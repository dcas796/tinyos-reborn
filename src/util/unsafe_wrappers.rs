use core::ops::Deref;

macro_rules! unsafe_wrapper {
    ($name:ident, $($t:ty),+) => {
        pub struct $name<T>(T);
        $(unsafe impl<T> $t for $name<T> {})+

        impl<T> $name<T> {
            pub const fn new(inner: T) -> Self {
                Self(inner)
            }

            pub fn into_inner(self) -> T {
                self.0
            }
        }

        impl<T> Deref for $name<T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        
        impl<T> From<T> for $name<T> {
            fn from(value: T) -> Self {
                Self(value)
            }
        }
    };
}

unsafe_wrapper!(UnsafeSync, Sync);
unsafe_wrapper!(UnsafeSend, Send);
unsafe_wrapper!(UnsafeSyncSend, Sync, Send);
