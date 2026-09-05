use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::cmp::min;
use core::fmt::Formatter;
use core::ops::Not;
use anyhow::bail;
use bitflags::bitflags;
use crate::fs;
use crate::fs::impls::FileImpl;
use crate::fs::path::{Path, PathBuf};
use crate::fs::vfs;
use crate::fs::vfs::{DirEntry, EntryKind, Metadata, SeekFrom};
use crate::io::hal;
use crate::util::date::Date;

const FAT_MAX_LENGTH_SHORT_NAME: usize = 8;
const FAT_MAX_LENGTH_SHORT_EXTENSION: usize = 3;
const FAT_NAME_LEN: usize = FAT_MAX_LENGTH_SHORT_NAME + FAT_MAX_LENGTH_SHORT_EXTENSION;

fn convert_to_83_filename_with_terminator(name: &[u8]) -> Option<[u8; FAT_NAME_LEN]> {
    let mut filename = [b' '; FAT_NAME_LEN];
    if let Some((name, extension)) = name.split_once(|b| *b == b'.') {
        if name.len() > FAT_MAX_LENGTH_SHORT_NAME || extension.len() > FAT_MAX_LENGTH_SHORT_EXTENSION {
            return None;
        }
        filename[..name.len()]
            .copy_from_slice(name);
        filename[FAT_MAX_LENGTH_SHORT_NAME..FAT_MAX_LENGTH_SHORT_NAME + extension.len()]
            .copy_from_slice(extension);
    } else {
        if name.len() > FAT_MAX_LENGTH_SHORT_NAME {
            return None;
        }
        filename[..name.len()].copy_from_slice(name);
    }

    filename.make_ascii_uppercase();
    Some(filename)
}

fn convert_from_83_filename(name: &[u8; FAT_NAME_LEN]) -> Vec<u8> {
    let mut filename = name[..FAT_MAX_LENGTH_SHORT_NAME]
        .iter()
        .take_while(|&&b| b != b' ')
        .copied()
        .collect::<Vec<u8>>();
    let extension = &name[FAT_MAX_LENGTH_SHORT_NAME..];

    if extension[0] != b' ' {
        filename.push(b'.');
        filename.extend(
            extension
                .iter()
                .take_while(|&&b| b != b' ')
                .copied()
        );
    }

    filename
}

fn fat_date_to_unix(date: u16, time: u16) -> Option<Date> {
    let year = ((date >> 9) & 0x7F) + 1980;
    let month = (date >> 5) & 0x0F;
    let day = date & 0x1F;

    let hour = (time >> 11) & 0x1F;
    let minute = (time >> 5) & 0x3F;
    let second = (time & 0x1F) * 2;

    Date::from_date_time(year, month as u8, day as u8, hour as u8, minute as u8, second as u8)
}

decoder_struct! {
    #[repr(C, packed)]
    struct FatBpb {
        jmp: [u8; 3],
        oem: [u8; 8],
        bytes_per_sector: u16,
        sectors_per_cluster: u8,
        reserved_sectors: u16,
        num_fats: u8,
        root_entry_count: u16,
        total_sectors_16: u16,
        media: u8,
        fat_size_16: u16,
        sectors_per_track: u16,
        num_heads: u16,
        hidden_sectors: u32,
        total_sectors_32: u32,
    }
}

decoder_struct! {
    #[repr(C, packed)]
    struct FatBpbExt12_16 {
        drive_number: u8,
        _reserved1: u8,
        boot_signature: u8,
        volume_id: u32,
        volume_label: [u8; 11],
        file_system_type: [u8; 8],
    }
}

decoder_struct! {
    #[repr(C, packed)]
    struct FatBpbExt32 {
        fat_size_32: u32,
        extended_flags: u16,
        fs_version: u16,
        root_cluster: u32,
        fs_info: u16,
        backup_boot_sector: u16,
        _reserved: [u8; 12],
        drive_number: u8,
        _reserved1: u8,
        boot_signature: u8,
        volume_id: u32,
        volume_label: [u8; 11],
        file_system_type: [u8; 8],
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct FatDirent {
    name: [u8; 11],
    attr: u8,
    ntres: u8,
    crt_time_tenth: u8,
    crt_time: u16,
    crt_date: u16,
    lst_acc_date: u16,
    fst_clus_hi: u16,
    wrt_time: u16,
    wrt_date: u16,
    fst_clus_lo: u16,
    file_size: u32,
}

impl FatDirent {
    fn root_dir(fst_cluster: u32) -> Self {
        let mut _self = Self {
            name: [b' '; 11],
            attr: 0,
            ntres: 0,
            crt_time_tenth: 0,
            crt_time: 0,
            crt_date: 0,
            lst_acc_date: 0,
            fst_clus_hi: 0,
            wrt_time: 0,
            wrt_date: 0,
            fst_clus_lo: 0,
            file_size: 0,
        };
        _self.set_attr(FatAttr::DIRECTORY);
        _self.set_fst_clus(fst_cluster);
        _self
    }

    fn attr(&self) -> FatAttr {
        FatAttr::from_bits_truncate(self.attr)
    }

    fn set_attr(&mut self, attr: FatAttr) {
        self.attr = attr.bits();
    }

    fn fst_clus(&self) -> u32 {
        ((self.fst_clus_hi as u32) << 16)  | self.fst_clus_lo as u32
    }

    fn set_fst_clus(&mut self, fst_clus: u32) {
        self.fst_clus_lo = fst_clus as u16;
        self.fst_clus_hi = (fst_clus >> 16) as u16;
    }

    fn to_dir_entry(self) -> Option<DirEntry> {
        self.attr().is_long_name().not().then(|| DirEntry {
            name: String::from_utf8(convert_from_83_filename(&self.name))
                .unwrap_or_else(|_| "<invalid>".to_string()),
            kind: if self.attr().contains(FatAttr::DIRECTORY) {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
        })
    }
}

decoder_struct! {
    #[repr(C, packed)]
    struct FatLfn {
        ord: u8,         /* sequence number (1..n), last has 0x40 set */
        name1: [u16; 5],    /* 5 UTF-16 chars */
        _attr: u8,        /* 0x0F */
        _type: u8,        /* 0 */
        chksum: u8,      /* checksum of associated short name */
        name2: [u16; 6],    /* 6 UTF-16 chars */
        _fst_clus_lo: u16, /* 0 */
        name3: [u16; 2],    /* 2 UTF-16 chars */
    }
}

bitflags! {
    struct FatAttr: u8 {
        const READ_ONLY = 0x01;
        const HIDDEN    = 0x02;
        const SYSTEM    = 0x04;
        const VOLUME_ID = 0x08;
        const DIRECTORY = 0x10;
        const ARCHIVE   = 0x20;
    }
}

impl FatAttr {
    fn is_long_name(&self) -> bool {
        self.contains(FatAttr::READ_ONLY | FatAttr::HIDDEN | FatAttr::SYSTEM | FatAttr::VOLUME_ID)
    }

    fn set_long_name(&mut self) {
        self.insert(FatAttr::READ_ONLY | FatAttr::HIDDEN | FatAttr::SYSTEM | FatAttr::VOLUME_ID);
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ClusterType {
    Valid,
    End,
    Bad,
    Unknown,
}

#[derive(Debug)]
enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

enum FatBpbExt {
    Fat12_16(FatBpbExt12_16),
    Fat32(FatBpbExt32),
}


pub struct FatFile<'a, 'b> {
    fat: &'b mut FatFileSystem<'a>,
    entry: FatDirent,
    offset: u32,
    path: PathBuf,
}

impl<'a, 'b> FatFile<'a, 'b> {
    fn new(fat: &'b mut FatFileSystem<'a>, entry: FatDirent, path: PathBuf) -> Self {
        Self {
            fat,
            entry,
            offset: 0,
            path,
        }
    }
}

impl vfs::File for FatFile<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> fs::Result<usize> {
        let bytes_per_cluster =
            self.fat.bpb.sectors_per_cluster as u32 * self.fat.bpb.bytes_per_sector as u32;
        let cluster_offset = self.offset / bytes_per_cluster;
        let mut start_cluster = self.entry.fst_clus();

        for _ in 0..cluster_offset {
            start_cluster = self.fat
                .next_cluster_not_bad(start_cluster)
                .map_err(|_| fs::error::Error::FileReadOutOfBounds)?;
        }

        let mut current_cluster = start_cluster;
        let mut current_offset = self.offset;
        let mut buf_offset = 0;
        let mut length = buf.len() as u32;
        while length > 0 {
            let offset_in_cluster = current_offset % bytes_per_cluster;
            let sector_offset = offset_in_cluster / self.fat.bpb.bytes_per_sector as u32;
            let length_to_read = min(length + offset_in_cluster, bytes_per_cluster) - offset_in_cluster;
            let sectors_to_read = length_to_read.div_ceil(self.fat.bpb.bytes_per_sector as u32) as u16;
            let byte_offset = current_offset % self.fat.bpb.bytes_per_sector as u32;

            let use_temp_buffer = byte_offset != 0 ||
                !length_to_read.is_multiple_of(self.fat.bpb.bytes_per_sector as u32);
            let buffer_size = sectors_to_read as u32 * self.fat.bpb.bytes_per_sector as u32;
            if use_temp_buffer {
                let mut temp_buffer = vec![0u8; buffer_size as usize];
                self.fat.disk.read_blocks(
                    self.fat.cluster_to_lba(current_cluster) + sector_offset as u64,
                    sectors_to_read as u32,
                    &mut temp_buffer,
                ).map_err(fs::error::Error::Other)?;
                buf[buf_offset..buf_offset + length_to_read as usize]
                    .copy_from_slice(&temp_buffer[
                        byte_offset as usize..byte_offset as usize + length_to_read as usize
                    ]);
            } else {
                self.fat.disk.read_blocks(
                    self.fat.cluster_to_lba(current_cluster) + sector_offset as u64,
                    sectors_to_read as u32,
                    &mut buf[buf_offset..buf_offset + buffer_size as usize],
                ).map_err(fs::error::Error::Other)?;
            };

            current_offset = 0;
            buf_offset += length_to_read as usize;
            length -= length_to_read;

            if length > 0 {
                current_cluster = self.fat
                    .next_cluster_not_bad(current_cluster)
                    .map_err(|_| fs::error::Error::FileReadOutOfBounds)?;
            }
        }

        let bytes_read = buf_offset;
        self.offset += bytes_read as u32;
        Ok(bytes_read)
    }

    fn write(&mut self, buf: &[u8]) -> fs::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> fs::Result<()> {
        todo!()
    }

    fn seek(&mut self, pos: SeekFrom) -> fs::Result<u64> {
        self.offset = match pos {
            SeekFrom::Start(o) => o
                .try_into()
                .map_err(|e| fs::error::Error::Other(anyhow::Error::from(e)))?,
            SeekFrom::End(o) => (self.entry.file_size as i64)
                .checked_add(o)
                .ok_or(fs::error::Error::FileReadOutOfBounds)?
                .checked_sub(1)
                .ok_or(fs::error::Error::FileReadOutOfBounds)?
                .try_into()
                .map_err(|e| fs::error::Error::Other(anyhow::Error::from(e)))?,
            SeekFrom::Current(o) => (self.offset as i64)
                .checked_add(o)
                .ok_or(fs::error::Error::FileReadOutOfBounds)?
                .try_into()
                .map_err(|e| fs::error::Error::Other(anyhow::Error::from(e)))?,
        };
        Ok(self.offset as u64)
    }

    fn metadata(&self) -> fs::Result<Metadata> {
        Ok(Metadata {
            path: self.path.clone(),
            kind: if self.entry.attr().contains(FatAttr::DIRECTORY) {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            size: self.entry.file_size as u64,
            creation_date: fat_date_to_unix(self.entry.crt_date, self.entry.crt_time)
                .unwrap_or_default(),
            last_modified_date: fat_date_to_unix(self.entry.wrt_date, self.entry.wrt_time)
                .unwrap_or_default(),
        })
    }

    fn truncate(&mut self, size: u64) -> fs::Result<()> {
        todo!()
    }
}

impl fmt::Display for FatFile<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FatFile(\"{}\")",
            String::from_utf8(convert_from_83_filename(&self.entry.name))
                .unwrap_or_else(|_| "<invalid>".to_string())
        )
    }
}

struct DirentIterator<'a, 'b> {
    fat: &'b mut FatFileSystem<'a>,
    current_cluster: Result<u32, ClusterType>,
    buffer: Vec<u8>,
    buffer_offset: usize,
}

impl<'a, 'b> DirentIterator<'a, 'b> {
    fn new(
        fat: &'b mut FatFileSystem<'a>,
        current_cluster: Result<u32, ClusterType>
    ) -> fs::Result<Self> {
        let buffer = if let Ok(current_cluster) = current_cluster {
            let sector_count = fat.bpb.sectors_per_cluster;
            let mut buffer = vec![
                0u8;
                sector_count as usize * fat.bpb.bytes_per_sector as usize
            ];
            fat.disk.read_blocks(
                fat.cluster_to_lba(current_cluster),
                sector_count as u32,
                &mut buffer,
            ).map_err(fs::error::Error::Other)?;
            buffer
        } else {
            Vec::new()
        };
        Ok(Self {
            fat,
            current_cluster,
            buffer,
            buffer_offset: 0,
        })
    }
}

impl<'a, 'b> Iterator for DirentIterator<'a, 'b> {
    type Item = fs::Result<FatDirent>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut current_cluster = self.current_cluster.ok()?;
        if self.buffer_offset + size_of::<FatDirent>() > self.buffer.len() {
            self.current_cluster = self.fat.next_cluster_not_bad(current_cluster);
            current_cluster = self.current_cluster.ok()?;
            match self.fat.disk.read_blocks(
                self.fat.cluster_to_lba(current_cluster),
                self.fat.bpb.sectors_per_cluster as u32,
                &mut self.buffer,
            ) {
                Ok(()) => {},
                Err(e) => return Some(Err(fs::error::Error::Other(e))),
            };
            self.buffer_offset = 0;
        }
        let dirent = unsafe {
            core::ptr::read(
                self.buffer[self.buffer_offset..].as_ptr() as *const FatDirent
            )
        };
        if dirent.name[0] == 0 {
            self.current_cluster = Err(ClusterType::End);
            return None;
        }
        self.buffer_offset += size_of::<FatDirent>();
        Some(Ok(dirent))
    }
}

pub struct FatFileSystem<'a> {
    disk: &'a mut dyn hal::disk::Disk,

    fat_type: FatType,
    first_fat_sector: u32,
    first_data_sector: u32,
    root_dir_sectors: u16,
    total_sectors: u32,
    cluster_count: u32,
    fat_size: u32,
    bpb: FatBpb,
    bpb_ext: FatBpbExt,

    // TODO: Maybe don't cache the ENTIRE FAT table, but only the ones that are really needed
    fat_cache: Vec<u8>,
}

impl<'a> FatFileSystem<'a> {
    pub fn detect(disk: &mut dyn hal::disk::Disk) -> anyhow::Result<bool> {
        let mut block = vec![0u8; disk.block_size()? as usize];
        disk.read_blocks(0, 1, &mut block)?;
        Ok(
            block[510] == 0x55 && block[511] == 0xAA &&
                (block[0] == 0xE9 || (block[0] == 0xEB && block[2] == 0x90))
        )
    }

    pub fn mount(disk: &'a mut dyn hal::disk::Disk) -> anyhow::Result<Self> {
        let mut buffer = vec![0u8; disk.block_size()? as usize];
        disk.read_blocks(0, 1, &mut buffer)?;

        let bpb = unsafe { FatBpb::from_bytes(&buffer) };
        let bpb_ext = {
            let bpb_ext_buffer = &buffer[size_of::<FatBpb>()..];
            if bpb.fat_size_16 != 0 {
                FatBpbExt::Fat12_16(unsafe {
                    FatBpbExt12_16::from_bytes(bpb_ext_buffer)
                })
            } else {
                FatBpbExt::Fat32(unsafe {
                    FatBpbExt32::from_bytes(bpb_ext_buffer)
                })
            }
        };

        let fat_size = match bpb_ext {
            FatBpbExt::Fat12_16(_) => bpb.fat_size_16 as u32,
            FatBpbExt::Fat32(ref ext) => ext.fat_size_32,
        };
        let total_sectors = if bpb.total_sectors_16 != 0 {
            bpb.total_sectors_16 as u32
        } else {
            bpb.total_sectors_32
        };
        let first_fat_sector = bpb.reserved_sectors as u32;
        let root_dir_sectors = if bpb.bytes_per_sector != 0 {
            (bpb.root_entry_count * 32).div_ceil(bpb.bytes_per_sector)
        } else {
            bail!("Corrupt FAT filesystem: invalid bytes per sector in BPB");
        };
        let first_data_sector =
            bpb.reserved_sectors as u32 + bpb.num_fats as u32 * fat_size + root_dir_sectors as u32;
        let cluster_count = if bpb.sectors_per_cluster != 0 {
            (total_sectors - first_data_sector) / bpb.sectors_per_cluster as u32
        } else {
            bail!("Corrupt FAT filesystem: invalid sectors per cluster in BPB");
        };
        let fat_type = match cluster_count {
            0..4085 => FatType::Fat12,
            4085..65525 => FatType::Fat16,
            _ => FatType::Fat32,
        };
        let mut fat_cache = vec![0u8; bpb.bytes_per_sector as usize * fat_size as usize];
        disk.read_blocks(first_fat_sector as u64, fat_size, &mut fat_cache)?;

        Ok(Self {
            disk,
            fat_type,
            first_fat_sector,
            first_data_sector,
            root_dir_sectors,
            total_sectors,
            cluster_count,
            fat_size,
            bpb,
            bpb_ext,
            fat_cache,
        })
    }

    fn oem_str(&self) -> &str {
        str::from_utf8(&self.bpb.oem).unwrap_or("<invalid>")
    }

    fn search_for_entry(&self, entries: &[FatDirent], name: &[u8]) -> fs::Result<FatDirent> {
        let converted_name = convert_to_83_filename_with_terminator(name)
            .ok_or(fs::error::Error::InvalidPath)?;

        for entry in entries {
            if entry.name[0] == 0 {
                break;
            }

            if entry.attr().is_long_name() {
                // TODO: implement LFN
            } else if entry.name == converted_name {
                return Ok(*entry)
            }
        }

        Err(fs::error::Error::FileNotFound)
    }

    fn search_root_for_entry(&mut self, name: &[u8]) -> fs::Result<FatDirent> {
        let first_root_dir_sector = self.first_data_sector - self.root_dir_sectors as u32;

        let sector_count = self.root_dir_sectors as u32;
        let buffer_size = sector_count * self.bpb.bytes_per_sector as u32;
        let mut buffer = vec![0u8; buffer_size as usize];
        self.disk
            .read_blocks(first_root_dir_sector as u64, sector_count, &mut buffer)
            .map_err(fs::error::Error::Other)?;
        let entries = unsafe {
            core::slice::from_raw_parts(
                buffer.as_ptr() as *const FatDirent,
                buffer.len() / size_of::<FatDirent>(),
            )
        };
        self.search_for_entry(entries, name)
    }

    fn cluster_to_lba(&self, cluster: u32) -> u64 {
        self.first_data_sector as u64 + (cluster as u64 - 2) * self.bpb.sectors_per_cluster as u64
    }

    fn get_cluster_type(&self, cluster: u32) -> ClusterType {
        match self.fat_type {
            FatType::Fat12 => match cluster {
                0x000..=0xFF6 => ClusterType::Valid,
                0xFF7 => ClusterType::Bad,
                0xFF8..=0xFFF => ClusterType::End,
                _ => ClusterType::Unknown,
            },
            FatType::Fat16 => match cluster {
                0x0000..=0xFFF6 => ClusterType::Valid,
                0xFFF7 => ClusterType::Bad,
                0xFFF8..=0xFFFF => ClusterType::End,
                _ => ClusterType::Unknown,
            },
            FatType::Fat32 => match cluster {
                0x00000000..=0x0FFFFFF6 => ClusterType::Valid,
                0x0FFFFFF7 => ClusterType::Bad,
                0x0FFFFFF8..=0x0FFFFFFF => ClusterType::End,
                _ => ClusterType::Unknown,
            }
        }
    }

    fn next_cluster(&self, cluster: u32) -> u32 {
        let fat_offset = match self.fat_type {
            FatType::Fat12 => cluster + cluster / 2,
            FatType::Fat16 => cluster * 2,
            FatType::Fat32 => cluster * 4,
        };

        match self.fat_type {
            FatType::Fat12 => {
                let value = u16::from_le_bytes([
                    self.fat_cache[fat_offset as usize],
                    self.fat_cache[fat_offset as usize + 1],
                ]);
                let next_cluster = if cluster & 1 != 0 {
                    value >> 4
                } else {
                    value & 0xFFF
                };
                next_cluster as u32
            },
            FatType::Fat16 => {
                let next_cluster = u16::from_le_bytes([
                    self.fat_cache[fat_offset as usize],
                    self.fat_cache[fat_offset as usize + 1],
                ]);
                next_cluster as u32
            },
            FatType::Fat32 => {
                u32::from_le_bytes([
                    self.fat_cache[fat_offset as usize],
                    self.fat_cache[fat_offset as usize + 1],
                    self.fat_cache[fat_offset as usize + 2],
                    self.fat_cache[fat_offset as usize + 3],
                ]) & 0x0FFFFFFF
            }
        }
    }

    fn next_cluster_not_bad(&self, cluster: u32) -> Result<u32, ClusterType> {
        let mut next_cluster = self.next_cluster(cluster);
        let mut cluster_type = self.get_cluster_type(next_cluster);
        while cluster_type == ClusterType::Bad {
            next_cluster = self.next_cluster(next_cluster);
            cluster_type = self.get_cluster_type(next_cluster);
        }
        if cluster_type == ClusterType::Valid {
            Ok(next_cluster)
        } else {
            Err(cluster_type)
        }
    }

    fn get_entries_in_directory(
        &mut self,
        dir: FatDirent
    ) -> fs::Result<impl Iterator<Item = fs::Result<FatDirent>> + '_> {
        let current_dir_cluster = {
            let current_dir_cluster = dir.fst_clus();
            match self.get_cluster_type(current_dir_cluster) {
                ClusterType::Valid => Ok(current_dir_cluster),
                t => Err(t)
            }
        };
        DirentIterator::new(self, current_dir_cluster)
    }

    fn get_directory_entry(&mut self, path: &Path) -> fs::Result<FatDirent> {
        let mut components = path.components();

        let mut current_directory = match self.bpb_ext {
            FatBpbExt::Fat12_16(_) => {
                let first_component = components
                    .next()
                    .ok_or(fs::error::Error::InvalidPath)?;
                self.search_root_for_entry(first_component.as_bytes())?
            }
            FatBpbExt::Fat32(ref bpb_ext) => {
                FatDirent::root_dir(bpb_ext.root_cluster)
            }
        };

        for component in components {
            let name = convert_to_83_filename_with_terminator(component.as_bytes())
                .ok_or(fs::error::Error::InvalidPath)?;
            current_directory = self.get_entries_in_directory(current_directory)?
                .find_map(|entry| match entry {
                    Ok(dirent) => if !dirent.attr().is_long_name() && dirent.name == name {
                        Some(Ok(dirent))
                    } else {
                        None
                    }
                    Err(e) => Some(Err(e))
                })
                .ok_or(fs::error::Error::FileNotFound)??;
        }

        Ok(current_directory)
    }
}

#[allow(refining_impl_trait)]
impl<'a> vfs::FileSystem for FatFileSystem<'a> {
    fn open(&mut self, path: &Path) -> fs::Result<FileImpl<'a, '_>> {
        path.is_absolute().ok_or(fs::error::Error::PathNotAbsolute)?;

        let directory_entry = self.get_directory_entry(path)?;

        if directory_entry.attr().contains(FatAttr::DIRECTORY) {
            return Err(fs::error::Error::IsDirectory);
        }

        Ok(FatFile::new(self, directory_entry, path.to_owned()).into())
    }

    fn create_file(&mut self, path: &Path) -> fs::Result<FileImpl<'a, '_>> {
        todo!()
    }

    fn remove_file(&mut self, path: &Path) -> fs::Result<()> {
        todo!()
    }

    fn create_dir(&mut self, path: &Path) -> fs::Result<()> {
        todo!()
    }

    fn remove_dir(&mut self, path: &Path) -> fs::Result<()> {
        todo!()
    }

    fn read_dir(&mut self, path: &Path) -> fs::Result<impl Iterator<Item = DirEntry>> {
        path.is_absolute().ok_or(fs::error::Error::PathNotAbsolute)?;

        let directory_entry = self.get_directory_entry(path)?;

        if !directory_entry.attr().contains(FatAttr::DIRECTORY) {
            return Err(fs::error::Error::NotADirectory);
        }

        self.get_entries_in_directory(directory_entry)?
            .try_fold(
                Vec::new(),
                |mut acc, entry| {
                    if let Some(dir_entry) = entry?.to_dir_entry() {
                        acc.push(dir_entry)
                    }
                    Ok(acc)
                }
            )
            .map(Vec::into_iter)
    }
}

impl fmt::Display for FatFileSystem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} filesystem, OEM: {}, total bytes: {}",
            self.fat_type,
            self.oem_str(),
            self.total_sectors * self.bpb.bytes_per_sector as u32
        )
    }
}
