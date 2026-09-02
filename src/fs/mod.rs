pub mod vfs;
pub mod error;
mod impls;
pub mod path;

use anyhow::bail;
pub use error::Result;
use crate::fs::impls::fat::FatFileSystem;
use crate::fs::impls::FileSystemImpl;
use crate::io::hal;

pub fn mount_filesystem<'a>(
    disk: &'a mut dyn hal::disk::Disk
) -> anyhow::Result<impl vfs::FileSystem + 'a> {
    if FatFileSystem::detect(disk)? {
        Ok(FileSystemImpl::Fat(FatFileSystem::mount(disk)?))
    } else {
        bail!("Unsupported filesystem")
    }
}
