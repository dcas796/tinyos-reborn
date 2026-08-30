use alloc::boxed::Box;
use core::fmt;

pub trait Disk: fmt::Display {
    fn read_blocks(&mut self, lba: u64, count: u32, buf: &mut [u8]) -> anyhow::Result<()>;
    fn write_blocks(&mut self, lba: u64, count: u32, buf: &[u8]) -> anyhow::Result<()>;
    fn flush(&mut self) -> anyhow::Result<()>;
    
    fn block_count(&mut self) -> anyhow::Result<u64>;
    fn block_size(&mut self) -> anyhow::Result<u32>;
    fn model(&mut self) -> anyhow::Result<&str>;
}

pub trait DiskController: fmt::Display {
    type Disk: Disk;

    fn is_disk_active(&self, index: usize) -> bool;
    fn active_disks(&self) -> Box<[usize]>;
    fn get_disk(&mut self, index: usize) -> Option<&mut Self::Disk>;
}
