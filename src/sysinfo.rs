#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct sysinfo_t {
    pub boot_drive: u8,
}
