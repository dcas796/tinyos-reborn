use alloc::boxed::Box;
use alloc::string::{String, ToString};
use core::ptr::NonNull;
use crate::{println, util};
use crate::vga::VgaColor::{End, LightRed};

static FADT_SIGNATURE: &[u8; 4] = b"FACP";

pub fn init_acpi(rsdp_ptr: *mut u8) -> Acpi {
    let rsdp_ptr = NonNull::new(rsdp_ptr).expect("RSDP pointer is null");
    let rsdp = unsafe { Rsdp::from_ptr(rsdp_ptr) };

    if rsdp.revision > 0 {
        println!("{LightRed}ACPI revision is {} (ACPI 2.0+). This may cause issues.{End}", rsdp.revision);
    }

    let rsdt = unsafe {
        Rsdt::from_ptr(
            NonNull::new(rsdp.rsdt_address as *mut u8)
                .expect("RSDT pointer is null")
        ).unwrap()
    };

    let (facp_ptr, _) = rsdt.find_table(FADT_SIGNATURE)
        .expect("FACP not found in RSDT");

    let fadt = unsafe { PartialFadt::from_ptr(facp_ptr) };

    Acpi {
        oem_id: str::from_utf8(&rsdp.oem_id).unwrap_or("Unknown").to_string(),
        revision: rsdp.revision,
        rsdt,
        fadt,
    }
}

pub struct Acpi {
    pub oem_id: String,
    pub revision: u8,
    pub rsdt: Rsdt,
    pub fadt: PartialFadt,
}

decoder_struct! {
    #[repr(C, packed)]
    pub struct Rsdp {
        pub signature: [u8; 8],
        pub checksum: u8,
        pub oem_id: [u8; 6],
        pub revision: u8,
        pub rsdt_address: u32,
    }
}

decoder_struct! {
    #[repr(C, packed)]
    pub struct SdtHeader {
        pub signature: [u8; 4],
        pub length: u32,
        pub revision: u8,
        pub checksum: u8,
        pub oem_id: [u8; 6],
        pub oem_table_id: [u8; 8],
        pub oem_revision: u32,
        pub creator_id: u32,
        pub creator_revision: u32,
    }
}

impl SdtHeader {
    pub fn verify_checksum(ptr: NonNull<u8>, length: usize) -> bool {
        let bytes = unsafe { core::slice::from_raw_parts(ptr.as_ptr(), length) };
        bytes.iter().fold(0u8, |acc, &x| acc.wrapping_add(x)) == 0
    }

    pub unsafe fn from_ptr_verifying_checksum(ptr: NonNull<u8>) -> Option<Self> {
        let header = unsafe { Self::from_ptr(ptr) };
        Self::verify_checksum(ptr, header.length as usize).then_some(header)
    }
}

pub struct Rsdt {
    pub header: SdtHeader,
    pub entries: Box<[u32]>,
}

impl Rsdt {
    pub unsafe fn from_ptr(ptr: NonNull<u8>) -> Result<Self, String> {
        let header = unsafe {
            SdtHeader::from_ptr_verifying_checksum(ptr)
                .ok_or("Invalid RSDT header checksum")?
        };
        let entries_ptr = unsafe {
            ptr.add(size_of::<SdtHeader>())
        };
        let entries_len = (header.length as usize - size_of::<SdtHeader>()) / size_of::<u32>();
        let entries = unsafe {
            util::slice::boxed_slice_from_nonaligned_ptr(entries_ptr.as_ptr(), entries_len)
        };
        Ok(Self { header, entries })
    }

    pub fn find_table(&self, signature: &[u8; 4]) -> Option<(NonNull<u8>, SdtHeader)> {
        self.entries
            .iter()
            .find_map(|&e| {
                let ptr = NonNull::new(e as *mut u8)?;
                let entry = unsafe { SdtHeader::from_ptr_verifying_checksum(ptr)? };
                (&entry.signature == signature).then_some((ptr, entry))
            })
    }
}

decoder_struct! {
    #[repr(C)]
    pub struct PartialFadt {
        pub firmware_ctrl: u32,
        pub dsdt: u32,
        pub reserved: u8,
        pub preferred_pmgmt_profile: u8,
        pub sci_interrupt: u16,
        pub smi_command_port: u32,
        pub acpi_enable: u8,
        pub acpi_disable: u8,
    }
}

impl PartialFadt {
    pub unsafe fn from_facp_ptr(ptr: NonNull<u8>) -> Self {
        unsafe { Self::from_ptr(ptr.add(size_of::<SdtHeader>())) }
    }
}
