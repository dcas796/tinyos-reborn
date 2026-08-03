macro_rules! decoder_struct {
    (
        #[repr($($repr:meta),*)]
        $(#[$attr:meta])*
        $vis:vis struct $name:ident {
            $($field_vis:vis $field:ident: $field_type:ty),*
            $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name {
            $($field_vis $field: $field_type),*
        }
        
        impl $name {
            pub unsafe fn from_ptr(ptr: ::core::ptr::NonNull<u8>) -> Self {
                #[repr($($repr),*)]
                $(#[$attr])*
                struct __repr {
                    $($field: $field_type),*
                }
                
                let repr = unsafe { ptr.cast::<__repr>().as_ref() };
                Self {
                    $($field: repr.$field),*
                }
            }
        }
    };
}
