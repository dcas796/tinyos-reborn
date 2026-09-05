use alloc::string::String;
use core::fmt;
use core::fmt::Formatter;
use enum_dispatch::enum_dispatch;
use crate::fs;
use crate::fs::path::{Path, PathBuf};
use crate::util::date::Date;

#[derive(Debug, Copy, Clone)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

#[derive(Debug, Copy, Clone)]
pub enum EntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub creation_date: Date,
    pub last_modified_date: Date,
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Metadata {{
    path: {:?}, 
    kind: {:?}, 
    size: {}, 
    creation_date: {}, 
    last_modified_date: {} 
}}",
            self.path, self.kind, self.size, self.creation_date, self.last_modified_date
        )
    }
}

#[enum_dispatch]
pub trait File: fmt::Display {
    fn read(&mut self, buf: &mut [u8]) -> fs::Result<usize>;
    fn write(&mut self, buf: &[u8]) -> fs::Result<usize>;
    fn flush(&mut self) -> fs::Result<()>;

    fn seek(&mut self, pos: SeekFrom) -> fs::Result<u64>;

    fn metadata(&self) -> fs::Result<Metadata>;

    fn truncate(&mut self, size: u64) -> fs::Result<()>;
}

#[enum_dispatch]
pub trait FileSystem: fmt::Display {
    fn open(&mut self, path: &Path) -> fs::Result<impl File + '_>;

    fn create_file(&mut self, path: &Path) -> fs::Result<impl File + '_>;
    fn remove_file(&mut self, path: &Path) -> fs::Result<()>;

    fn create_dir(&mut self, path: &Path) -> fs::Result<()>;
    fn remove_dir(&mut self, path: &Path) -> fs::Result<()>;

    fn read_dir(&mut self, path: &Path) -> fs::Result<impl Iterator<Item = DirEntry>>;
}
