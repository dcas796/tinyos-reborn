use core::mem;
use x86::Ring;
use x86::segmentation::{SegmentSelector, SystemDescriptorTypes32};

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    zero: u8,
    flags: u8,
    offset_high: u16
}

impl IdtEntry {
    pub const fn new(
        offset: u32,
        selector: SegmentSelector,
        gate_type: SystemDescriptorTypes32,
        dpl: Ring,
    ) -> Self {
        Self {
            offset_low: (offset & 0xFFFF) as u16,
            selector: selector.bits(),
            zero: 0,
            flags: ((gate_type as u8) & 0b1111) | ((dpl as u8) << 5) | 0x80, // Present bit
            offset_high: ((offset >> 16) & 0xFFFF) as u16,
        }
    }

    pub const fn from_u64(entry: u64) -> Option<Self> {
        if entry & 0x80 == 0 {
            None
        } else {
            unsafe {
                Some(mem::transmute::<u64, IdtEntry>(entry))
            }
        }
    }
    
    pub const fn into_u64(self) -> u64 {
        unsafe {
            mem::transmute::<IdtEntry, u64>(self)
        }
    }
}

impl From<IdtEntry> for u64 {
    fn from(value: IdtEntry) -> Self {
        value.into_u64()
    }
}
