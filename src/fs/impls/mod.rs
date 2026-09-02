pub mod fat;

use crate::fs::path::Path;
use crate::fs::vfs::Metadata;
use crate::fs::vfs::SeekFrom;
use alloc::boxed::Box;
use core::fmt;
use core::fmt::{Formatter, Pointer};
use crate::fs;
use crate::fs::vfs::DirEntry;
use crate::fs::vfs::File;
use crate::fs::vfs::FileSystem;
use enum_dispatch::enum_dispatch;

#[enum_dispatch(File)]
pub enum FileImpl<'a, 'b> {
    Fat(fat::FatFile<'a, 'b>),
}

impl fmt::Display for FileImpl<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FileImpl::Fat(fat_file) => fat_file.fmt(f),
        }
    }
}

#[enum_dispatch(FileSystem)]
pub enum FileSystemImpl<'a> {
    Fat(fat::FatFileSystem<'a>),
}

impl fmt::Display for FileSystemImpl<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FileSystemImpl::Fat(fat_fs) => fat_fs.fmt(f),
        }
    }
}
