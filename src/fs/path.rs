#[repr(transparent)]
pub struct Path {
    inner: str,
}

impl Path {
    pub const fn new(s: &str) -> &Self {
        unsafe { &*(s as *const str as *const Path) }
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.inner
            .strip_prefix("/")
            .unwrap_or(&self.inner)
            .split("/")
    }

    pub fn is_absolute(&self) -> bool {
        self.inner.starts_with("/")
    }
}
