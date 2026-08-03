use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr::NonNull;
use int_enum::IntEnum;
use crate::io::acpi::{Rsdt, SdtHeader};
use crate::{logln, util};

static MCFG_SIGNATURE: &[u8; 4] = b"MCFG";

pub fn init_pci(rsdt: &Rsdt) -> Result<Pci, String> {
    let (mcfg_ptr, mcfg_header) = rsdt
        .find_table(MCFG_SIGNATURE)
        .ok_or("Could not initialize PCI: MCFG not found in RSDT")?;

    let entries_ptr = unsafe { mcfg_ptr.as_ptr().add(size_of::<SdtHeader>() + 8) };
    let entries_len =
        (mcfg_header.length as usize - size_of::<SdtHeader>() - 8) / size_of::<ConfigSpaceEntry>();
    let entries = unsafe {
        util::slice::boxed_slice_from_nonaligned_ptr::<ConfigSpaceEntry>(entries_ptr, entries_len)
    };

    logln!("Found {} PCI configuration space entries: {entries:#x?}", entries.len());

    let config_space = ConfigSpace {
        entries,
    };

    let endpoints = config_space.enumerate_endpoints();

    Ok(Pci {
        config_space,
        endpoints,
    })
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Vendor(u16);

impl Vendor {
    const INVALID_PCI_VENDOR: u16 = 0xFFFF;

    pub fn from_word(id: u16) -> Option<Self> {
        (id != Self::INVALID_PCI_VENDOR).then_some(Self(id))
    }

    pub fn id(&self) -> u16 {
        self.0
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, IntEnum)]
pub enum HeaderType {
    GeneralDevice      = 0,
    Pci2PciBridge      = 1,
    Pci2CardBusBridge  = 2,
}

impl HeaderType {
    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::try_from(byte & 0b0111_1111).ok()
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct ConfigSpaceEntry {
    base_address: u64,
    segment_group_number: u16,
    start_bus_number: u8,
    end_bus_number: u8,
    reserved: [u8; 4],
}

#[derive(Debug)]
pub struct ConfigSpace {
    entries: Box<[ConfigSpaceEntry]>,
}

macro_rules! config_read_func {
    ($name:ident, $t:ty) => {
        pub fn $name(&self, segment_group: u16, bus: u8, device: u8, func: u8, offset: u16) -> $t {
            unsafe { self.config_read(segment_group, bus, device, func, offset) }
        }
    };
}

macro_rules! get_header_field {
    (
        name = $name:ident,
        offset = $offset:expr,
        read_func = $read:expr,
        return_type = $ret:ty
        $(,map = $map:expr)?
    ) => {
        pub fn $name(&self, segment_group: u16, bus: u8, device: u8, func: u8) -> $ret {
            $($map)?($read(self, segment_group, bus, device, func, $offset))
        }
    };
}

impl ConfigSpace {
    const MULTIPLE_FUNCTIONS_MASK: u8 = 0b1000_0000;

    #[inline]
    fn get_address(&self, segment_group: u16, bus: u8, device: u8, func: u8, offset: u16) -> u32 {
        let bus = bus as u32;
        let device = device as u32 & 0b0001_1111;
        let func = func as u32 & 0b0000_0111;
        let offset = offset as u32 & 0b0000_1111_1111_1111;
        let base_address = self.entries[segment_group as usize].base_address as u32;
        base_address + ((bus << 20) | (device << 15) | (func << 12) | offset)
    }

    #[inline]
    unsafe fn config_read<T: Copy>(&self, segment_group: u16, bus: u8, device: u8, func: u8, offset: u16) -> T {
        unsafe { *(self.get_address(segment_group, bus, device, func, offset) as *const T) }
    }

    config_read_func!(config_read_byte, u8);
    config_read_func!(config_read_word, u16);

    get_header_field!(
        name = get_vendor, offset = 0x000, read_func = Self::config_read_word,
        return_type = Option<Vendor>, map = Vendor::from_word
    );

    get_header_field!(
        name = get_header_type, offset = 0x00E, read_func = Self::config_read_byte,
        return_type = (u8, Option<HeaderType>), map = |byte| (byte, HeaderType::from_byte(byte))
    );

    pub fn is_device_present(&self, segment_group: u16, bus: u8, device: u8) -> bool {
        self.get_vendor(segment_group, bus, device, 0).is_some()
    }

    pub fn has_multiple_functions(&self, segment_group: u16, bus: u8, device: u8) -> bool {
        self.get_header_type(segment_group, bus, device, 0).0 & Self::MULTIPLE_FUNCTIONS_MASK != 0
    }
}

decoder_struct! {
    #[repr(C, packed)]
    struct CommonHeader {
        vendor_id: u16,
        device_id: u16,
        command: u16,
        status: u16,
        revision_id: u8,
        prog_if: u8,
        subclass: u8,
        class_code: u8,
        cache_line_size: u8,
        latency_timer: u8,
        header_type: u8,
        bist: u8,
    }
}

decoder_struct! {
    #[repr(C, packed)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct GeneralDeviceHeader {
        pub bar0: u32,
        pub bar1: u32,
        pub bar2: u32,
        pub bar3: u32,
        pub bar4: u32,
        pub bar5: u32,
        pub cardbus_cis_pointer: u32,
        pub subsystem_vendor_id: u16,
        pub subsystem_id: u16,
        pub expansion_rom_base_address: u32,
        pub capabilities_pointer: u8,
        pub reserved0: [u8; 7],
        pub interrupt_line: u8,
        pub interrupt_pin: u8,
        pub min_grant: u8,
        pub max_latency: u8
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExtendedHeader {
    GeneralDevice(GeneralDeviceHeader),
    /* TODO: Maybe add the rest of headers?
    Pci2PciBridge(Pci2PciBridgeHeader),
    Pci2CardBusBridge(Pci2CardBusBridgeHeader),
    */
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FunctionIdentifier {
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub segment_group: u16,
    pub bus: u8,
    pub device: u8,
    pub func: u8,

    pub vendor: Vendor,
    pub device_id: u16,
    pub revision_id: u8,
    pub func_identifier: FunctionIdentifier,

    pub extended_header: ExtendedHeader,
}

macro_rules! get_struct {
    (
        name = $name:ident,
        offset = $offset:expr,
        struct_type = $struct_type:tt
    ) => {
        unsafe fn $name(
            &self,
            segment_group: u16,
            bus: u8,
            device: u8,
            func: u8,
        ) -> $struct_type {
            let address = self.get_address(
                segment_group, bus, device, func, $offset
            );
            let ptr = NonNull::new(address as *mut u8).unwrap();
            unsafe { $struct_type::from_ptr(ptr) }
        }
    };
}

impl ConfigSpace {
    const MULTIPLE_FUNCTIONS_NUM: u8 = 8;

    get_struct!(name = get_common_header, offset = 0x000, struct_type = CommonHeader);
    get_struct!(name = get_general_device_header, offset = 0x010, struct_type = GeneralDeviceHeader);

    #[inline]
    fn enumerate_device(
        &self,
        segment_group: u16,
        bus: u8,
        device: u8,
        endpoints: &mut Vec<Endpoint>
    ) {
        if !self.is_device_present(segment_group, bus, device) {
            return;
        }

        let num_funcs = if self.has_multiple_functions(segment_group, bus, device) {
            Self::MULTIPLE_FUNCTIONS_NUM
        } else {
            1
        };

        for func in 0..num_funcs {
            if !self.is_device_present(segment_group, bus, device) {
                continue;
            }

            let vendor = match self.get_vendor(segment_group, bus, device, func) {
                Some(vendor) => vendor,
                None => continue,
            };

            let common_header = unsafe {
                self.get_common_header(segment_group, bus, device, func)
            };

            let header_type = HeaderType::from_byte(common_header.header_type)
                .unwrap_or_else(|| {
                    panic!(
                        "Invalid header type {:#x} for endpoint {segment_group}:{bus}:{device}.{func}",
                        common_header.header_type
                    )
                });

            let extended_header = match header_type {
                HeaderType::GeneralDevice => ExtendedHeader::GeneralDevice(unsafe {
                    self.get_general_device_header(segment_group, bus, device, func)
                }),
                _ => {
                    logln!("Warning: Unsupported header type {header_type:?} for endpoint {segment_group}:{bus}:{device}.{func}. Ignoring...");
                    continue;
                },
            };

            endpoints.push(Endpoint {
                segment_group,
                bus,
                device,
                func,
                vendor,
                device_id: common_header.device_id,
                revision_id: common_header.revision_id,
                func_identifier: FunctionIdentifier {
                    class: common_header.class_code,
                    subclass: common_header.subclass,
                    prog_if: common_header.prog_if,
                },
                extended_header,
            });
        }
    }

    fn enumerate_endpoints(&self) -> Box<[Endpoint]> {
        let mut endpoints = Vec::new();

        for entry in self.entries.iter() {
            for bus in entry.start_bus_number..=entry.end_bus_number {
                for device in 0..32 {
                    self.enumerate_device(entry.segment_group_number, bus, device, &mut endpoints)
                }
            }
        }

        endpoints.into_boxed_slice()
    }
}

#[derive(Debug)]
pub struct Pci {
    pub config_space: ConfigSpace,
    pub endpoints: Box<[Endpoint]>,
}
