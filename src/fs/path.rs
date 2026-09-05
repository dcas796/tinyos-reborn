use alloc::borrow::ToOwned;
use alloc::fmt;
use alloc::string::String;
use core::borrow::Borrow;
use core::fmt::Formatter;

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

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> Self::Owned {
        PathBuf::new(self.inner.to_owned())
    }
}

#[derive(Clone)]
pub struct PathBuf {
    inner: String,
}

impl PathBuf {
    pub const fn new(s: String) -> Self {
        Self {
            inner: s
        }
    }
}

impl fmt::Debug for PathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        Path::new(self.inner.borrow())
    }
}
