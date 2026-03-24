#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub enum sysinfo_mem_type_t {
    SYSINFO_MT_USABLE,
    SYSINFO_MT_RECLAIMABLE,
    SYSINFO_MT_ELF_EXECUTABLE,
    SYSINFO_MT_ACPI_NVS,
    SYSINFO_MT_BAD,
    SYSINFO_MT_RESERVED,
}

use core::ptr::NonNull;
pub use sysinfo_mem_type_t::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct sysinfo_memregion_t {
    pub next: *mut sysinfo_memregion_t,
    pub base_addr: u64,
    pub size: u64,
    pub mtype: sysinfo_mem_type_t,
    pub is_volatile: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct sysinfo_t {
    pub boot_drive: u8,
    pub mem_regions: *mut sysinfo_memregion_t,
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MemoryType {
    Usable,
    Reclaimable,
    ElfExecutable,
    AcpiNVS,
    Bad,
    Reserved,
}

impl From<sysinfo_mem_type_t> for MemoryType {
    fn from(value: sysinfo_mem_type_t) -> Self {
        match value {
            SYSINFO_MT_USABLE => MemoryType::Usable,
            SYSINFO_MT_RECLAIMABLE => MemoryType::Reclaimable,
            SYSINFO_MT_ELF_EXECUTABLE => MemoryType::ElfExecutable,
            SYSINFO_MT_ACPI_NVS => MemoryType::AcpiNVS,
            SYSINFO_MT_BAD => MemoryType::Bad,
            SYSINFO_MT_RESERVED => MemoryType::Reserved,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MemoryRegion {
    pub base_addr: u64,
    pub size: u64,
    pub region_type: MemoryType,
    pub is_volatile: bool,
}

impl From<&sysinfo_memregion_t> for MemoryRegion {
    fn from(value: &sysinfo_memregion_t) -> Self {
        Self {
            base_addr: value.base_addr,
            size: value.size,
            region_type: value.mtype.into(),
            is_volatile: value.is_volatile,
        }
    }
}

pub struct MemoryRegions {
    first: *mut sysinfo_memregion_t,
}

impl MemoryRegions {
    pub fn iter(&self) -> impl Iterator<Item = MemoryRegion> {
        MemoryRegionsIter::from(self)
    }
}

impl From<*mut sysinfo_memregion_t> for MemoryRegions {
    fn from(value: *mut sysinfo_memregion_t) -> Self {
        Self {
            first: value,
        }
    }
}

impl IntoIterator for MemoryRegions {
    type Item = MemoryRegion;
    type IntoIter = MemoryRegionsIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into()
    }
}

pub struct MemoryRegionsIter {
    current: *mut sysinfo_memregion_t,
}

impl From<&MemoryRegions> for MemoryRegionsIter {
    fn from(value: &MemoryRegions) -> Self {
        Self {
            current: value.first,
        }
    }
}

impl Iterator for MemoryRegionsIter {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        let current = unsafe { NonNull::new(self.current)?.as_ref() };
        let region: MemoryRegion = current.into();
        self.current = current.next;
        Some(region)
    }
}
