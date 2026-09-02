use alloc::string::String;
use core::fmt;
use enum_dispatch::enum_dispatch;
use crate::fs;
use crate::fs::path::Path;

#[derive(Debug, Copy, Clone)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(u64),
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

#[derive(Debug, Copy, Clone)]
pub struct Metadata {
    pub kind: EntryKind,
    pub size: u64,
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
