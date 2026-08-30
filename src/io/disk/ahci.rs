use alloc::alloc::alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::alloc::{Layout};
use core::{fmt, slice};
use core::cell::RefCell;
use crate::io::hal;
use crate::io::pci;
use crate::{int, volatile};
use crate::interrupt::{pic, register_irq};
use crate::interrupt::regs::Registers;
use crate::util::interrupt_lock::InterruptLock;

// GHC bits
const GHC_AE: u32 = 1 << 31; // AHCI enable
const GHC_IE: u32 = 1 << 1;  // Interrupt enable
const GHC_HR: u32 = 1 << 0;  // HBA reset

// PxCMD bits
const PXCMD_ST: u32 = 1 << 0;  // Start
const PXCMD_FRE: u32 = 1 << 4; // FIS receive enable
const PXCMD_FR: u32 = 1 << 14; // FIS receive running
const PXCMD_CR: u32 = 1 << 15; // Command list running

// PxSSTS.DET values
const DET_PRESENT: u32 = 0x3;
// PxSSTS.IPM values
const IPM_ACTIVE: u32 = 0x1;

// PxTFD error/busy bits
const TFD_ERR: u32 = 1 << 0;
const TFD_DRQ: u32 = 1 << 3;
const TFD_BSY: u32 = 1 << 7;

// PxIS bits worth checking
const PXIS_TFES: u32 = 1 << 30; // Task File Error Status

// PxIE bits
const PXIE_DHRE: u32 = 1 << 0;  // D2H Register FIS Interrupt Enable
const PXIE_PSE: u32  = 1 << 1;  // PIO Setup FIS Interrupt Enable
const PXIE_DSE: u32  = 1 << 2;  // DMA Setup FIS Interrupt Enable
const PXIE_TFEE: u32 = 1 << 30; // Task File Error Interrupt Enable

const HBA_PORTS_COUNT: u32 = 32;

const PRDT_LEN: u16 = 8;
const PRDT_MAX_DATA_BYTES: u32 = (1 << 23) - 1;

const FIS_TYPE_REG_H2D: u8 = 0x27;

const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_READ_DMA_EX: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EX: u8 = 0x35;
const ATA_CMD_FLUSH_EX: u8 = 0xEA;

static AHCI_TASK_MANAGER: InterruptLock<RefCell<AhciTaskManager>> =
    InterruptLock::new(RefCell::new(AhciTaskManager::new()));

int! {
    #[irq(pic::current_irq().expect("Spurious IRQ"))]
    fn ahci_irq_handler(_regs: Registers) {
        let manager = AHCI_TASK_MANAGER.get();
        let controllers = manager.borrow().controllers().to_vec();
        for (id, hba_ptr) in controllers {
            let hba = unsafe { &mut *hba_ptr };
            let is = hba.is.read();
            if is == 0 {
                continue;
            }

            for port_index in 0..HBA_PORTS_COUNT {
                if is & (1 << port_index) == 0 {
                    continue;
                }

                let port = &mut hba.ports.get_mut()[port_index as usize];
                let pxis = port.is.read();
                let tfd = port.tfd.read();

                port.is.write(pxis);

                let ci = port.ci.read();
                let sact = port.sact.read();
                let outstanding = ci | sact;

                let status = if pxis & PXIS_TFES != 0 || tfd & TFD_ERR != 0 {
                    AhciTaskStatus::Failed
                } else {
                    AhciTaskStatus::Success
                };

                let mut manager = manager.borrow_mut();
                for slot in 0usize..32 {
                    if outstanding & (1 << slot) != 0 {
                        continue;
                    }

                    let task = AhciTask {
                        controller: id,
                        port: port_index,
                        slot,
                    };

                    if manager.poll(&task) == Some(AhciTaskStatus::Pending) {
                        manager.complete_task(task, status);
                    }
                }
            }

            hba.is.write(is);
        }
    }
}

// https://wiki.osdev.org/AHCI

volatile! {
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct HbaPort {
        clb: u32,          // 0x00, command list base address, 1K-byte aligned
        clbu: u32,         // 0x04, command list base address upper 32 bits
        fb: u32,           // 0x08, FIS base address, 256-byte aligned
        fbu: u32,          // 0x0C, FIS base address upper 32 bits
        is: u32,           // 0x10, interrupt status
        ie: u32,           // 0x14, interrupt enable
        cmd: u32,          // 0x18, command and status
        rsv0: u32,         // 0x1C, Reserved
        tfd: u32,          // 0x20, task file data
        sig: u32,          // 0x24, signature
        ssts: u32,         // 0x28, SATA status (SCR0:SStatus)
        sctl: u32,         // 0x2C, SATA control (SCR2:SControl)
        serr: u32,         // 0x30, SATA error (SCR1:SError)
        sact: u32,         // 0x34, SATA active (SCR3:SActive)
        ci: u32,           // 0x38, command issue
        sntf: u32,         // 0x3C, SATA notification (SCR4:SNotification)
        fbs: u32,          // 0x40, FIS-based switch control
        _rsv1: [u32; 11],  // 0x44 ~ 0x6F, Reserved
        _vendor: [u32; 4], // 0x70 ~ 0x7F, vendor specific
    }
}

volatile! {
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct HbaMem {
        // 0x00 - 0x2B, Generic Host Control
        cap: u32,      // 0x00, Host capability
        ghc: u32,      // 0x04, Global host control
        is: u32,       // 0x08, Interrupt status
        pi: u32,       // 0x0C, Port implemented
        vs: u32,       // 0x10, Version
        ccc_ctl: u32,  // 0x14, Command completion coalescing control
        ccc_pts: u32,  // 0x18, Command completion coalescing ports
        em_loc: u32,   // 0x1C, Enclosure management location
        em_ctl: u32,   // 0x20, Enclosure management control
        cap2: u32,     // 0x24, Host capabilities extended
        bohc: u32,     // 0x28, BIOS/OS handoff control and status

        // 0x2C - 0x9F, Reserved
        _rsv: [u8; 0xA0-0x2C],

        // 0xA0 - 0xFF, Vendor specific registers
        _vendor: [u8; 0x100-0xA0],

        // 0x100 - 0x10FF, Port control registers
        ports: [HbaPort; HBA_PORTS_COUNT as usize],  // 1 ~ 32
    }
}

#[derive(Copy, Clone)]
struct HbaCmdHeaderFlags {
    /// Command FIS length in DWORDS (2 ~ 16)
    cfl: u8,
    /// ATAPI
    a: bool,
    /// Write, true: H2D, false: D2H
    w: bool,
    /// Prefetchable
    p: bool,
    /// Reset
    r: bool,
    /// BIST
    b: bool,
    /// Clear busy upon R_OK
    c: bool,
    /// Port multiplier port (0 ~ 15)
    pmp: u8
}

impl HbaCmdHeaderFlags {
    const fn from_bits(bits: u16) -> Self {
        Self {
            cfl: (bits & 0x1F) as u8,
            a: (bits & (1 << 5)) != 0,
            w: (bits & (1 << 6)) != 0,
            p: (bits & (1 << 7)) != 0,
            r: (bits & (1 << 8)) != 0,
            b: (bits & (1 << 9)) != 0,
            c: (bits & (1 << 10)) != 0,
            pmp: ((bits >> 12) & 0xF) as u8
        }
    }

    const fn to_bits(self) -> u16 {
        (self.cfl as u16 & 0x1F) |
        ((self.a as u16) << 5) |
        ((self.w as u16) << 6) |
        ((self.p as u16) << 7) |
        ((self.r as u16) << 8) |
        ((self.b as u16) << 9) |
        ((self.c as u16) << 10) |
        ((self.pmp as u16 & 0xF) << 12)
    }
}

volatile! {
    #[repr(C)]
    struct HbaCmdHeader {
        // byte 0: cfl(5) a(1) w(1) p(1), byte 1: r(1) b(1) c(1) rsv(1) pmp(4)
        flags: u16,
        prdtl: u16,       // Physical region descriptor table length (entries)
        prdbc: u32,       // PRD byte count transferred (set by HBA)
        ctba: u32,        // Command table base address, low, 128-byte aligned
        ctbau: u32,       // Command table base address, high
        _reserved: [u32; 4],
    }
}

volatile! {
    #[repr(C)]
    struct HbaPrdtEntry {
        dba: u32,   // Data base address, low, 2-byte aligned
        dbau: u32,  // Data base address, high
        _reserved0: u32,
        dbc_and_i: u32,   // bits 0..21 = byte count - 1 (must be even), bit 31 = interrupt on completion
    }
}

impl HbaPrdtEntry {
    fn read_dbc(&self) -> u32 {
        self.dbc_and_i.read() & 0x003F_FFFF
    }

    fn write_dbc(&mut self, dbc: u32) {
        self.dbc_and_i.write(self.dbc_and_i.read() & !0x003F_FFFF | (dbc & 0x003F_FFFF));
    }

    fn read_i(&self) -> bool {
        self.dbc_and_i.read() & 0x8000_0000 != 0
    }

    fn write_i(&mut self, i: bool) {
        self.dbc_and_i.write(self.dbc_and_i.read() & !0x8000_0000 | ((i as u32) << 31));
    }
}

volatile! {
    #[repr(C)]
    struct HbaCmdTable {
        cfis: [u8; 64],   // Command FIS
        acmd: [u8; 16],   // ATAPI command, unused for ATA disks
        _reserved: [u8; 48],
        prdt: [HbaPrdtEntry; PRDT_LEN as usize],
    }
}

#[repr(C)]
#[derive(Default)]
struct FisRegH2D {
    // Always FIS_TYPE_REG_D2H
    fis_type: u8,
    // Port multiplier and command/control bit
    pmport_and_c: u8,
    // Command register
    command: u8,
    // Feature register (7:0)
    featurel: u8,
    // LBA low register (7:0)
    lba0: u8,
    // LBA mid register (15:8)
    lba1: u8,
    // LBA high register (23:16)
    lba2: u8,
    // Device register
    device: u8,
    // LBA register (31:24)
    lba3: u8,
    // LBA register (39:32)
    lba4: u8,
    // LBA register (47:40)
    lba5: u8,
    // Feature register (15:8)
    featureh: u8,
    // Count register (7:0)
    countl: u8,
    // Count register (15:8)
    counth: u8,
    // Isochronous command completion
    icc: u8,
    // Control register
    control: u8,
    // Reserved
    _rsv1: [u8; 4],
}

impl FisRegH2D {
    fn pmport(&self) -> u8 {
    self.pmport_and_c & 0x0F
    }

    fn set_pmport(&mut self, pmport: u8) {
        self.pmport_and_c = (self.pmport_and_c & 0xF0) | (pmport & 0x0F);
    }

    /// true: Command, false: Control
    fn c(&self) -> bool {
        (self.pmport_and_c & 0x80) != 0
    }

    /// true: Command, false: Control
    fn set_c(&mut self, c: bool) {
        self.pmport_and_c = (self.pmport_and_c & 0x7F) | ((c as u8) << 7);
    }

    fn feature(&self) -> u16 {
        (self.featurel as u16) | ((self.featureh as u16) << 8)
    }

    fn set_feature(&mut self, feature: u16) {
        self.featurel = feature as u8;
        self.featureh = (feature >> 8) as u8;
    }

    fn lba(&self) -> u64 {
        (self.lba0 as u64) |
        ((self.lba1 as u64) << 8) |
        ((self.lba2 as u64) << 16) |
        ((self.lba3 as u64) << 24) |
        ((self.lba4 as u64) << 32) |
        ((self.lba5 as u64) << 40)
    }

    fn set_lba(&mut self, lba: u64) {
        self.lba0 = lba as u8;
        self.lba1 = (lba >> 8) as u8;
        self.lba2 = (lba >> 16) as u8;
        self.lba3 = (lba >> 24) as u8;
        self.lba4 = (lba >> 32) as u8;
        self.lba5 = (lba >> 40) as u8;
    }

    fn count(&self) -> u16 {
        (self.countl as u16) | ((self.counth as u16) << 8)
    }

    fn set_count(&mut self, count: u16) {
        self.countl = count as u8;
        self.counth = (count >> 8) as u8;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AhciControllerIdentifier {
    segment_group: u16,
    bus: u8,
    device: u8,
    func: u8,
}

impl AhciControllerIdentifier {
    const fn new(endpoint: &pci::Endpoint) -> Self {
        let &pci::Endpoint { segment_group, bus, device, func, .. } = endpoint;
        Self {
            segment_group,
            bus,
            device,
            func
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AhciTask {
    controller: AhciControllerIdentifier,
    port: u32,
    slot: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AhciTaskStatus {
    Pending,
    Success,
    Failed,
}

struct AhciTaskManager {
    tasks: BTreeMap<AhciTask, AhciTaskStatus>,
    controllers: Vec<(AhciControllerIdentifier, *mut HbaMem)>,
}

impl AhciTaskManager {
    const fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            controllers: Vec::new(),
        }
    }

    fn register_controller(&mut self, id: AhciControllerIdentifier, hba: *mut HbaMem) {
        self.controllers.push((id, hba));
    }

    fn register_task(&mut self, task: AhciTask) {
        self.tasks.insert(task, AhciTaskStatus::Pending);
    }

    fn poll(&self, task: &AhciTask) -> Option<AhciTaskStatus> {
        self.tasks.get(task).copied()
    }

    fn complete_task(&mut self, task: AhciTask, status: AhciTaskStatus) {
        self.tasks.insert(task, status);
    }

    fn remove_task(&mut self, task: &AhciTask) {
        self.tasks.remove(task);
    }

    fn controllers(&self) -> &[(AhciControllerIdentifier, *mut HbaMem)] {
        &self.controllers
    }
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
enum AhciDiskError {
    #[error("No free slot for the ATA command found")]
    NoFreeSlot,
    #[error("ATA command failed")]
    TaskFailed,
    #[error("Output buffer is too small for the requested amount of data")]
    BufferTooSmall,
    #[error("Output buffer is too big")]
    BufferTooBig,
}

#[derive(Copy, Clone)]
struct DiskInfo {
    block_count: u64,
    block_size: u32,
    model: [u8; 40],
}

pub struct AhciDisk {
    controller: AhciControllerIdentifier,
    port_index: u32,
    port_ptr: *mut HbaPort,
    info: Option<DiskInfo>,
}

impl AhciDisk {
    const CLB_LAYOUT: Layout = match Layout::from_size_align(1024, 1024) {
        Ok(layout) => layout,
        Err(_) => panic!("Failed to create CLB layout"),
    };
    const FB_LAYOUT: Layout = match Layout::from_size_align(256, 256) {
        Ok(layout) => layout,
        Err(_) => panic!("Failed to create FB layout"),
    };
    const CMD_TABLE_LAYOUT: Layout = match Layout::from_size_align(size_of::<HbaCmdTable>(), 128) {
        Ok(layout) => layout,
        Err(_) => panic!("Failed to create CMD_TABLE layout"),
    };

    const DEFAULT_BLOCK_SIZE: u32 = 512;

    fn new(controller: AhciControllerIdentifier, port_index: u32, port_ptr: *mut HbaPort) -> Self {
        let mut _self = Self {
            controller,
            port_index,
            port_ptr,
            info: None,
        };

        _self.stop_commands();

        let port = unsafe { _self.port_mut() };
        let new_clb = unsafe { alloc(Self::CLB_LAYOUT) };
        assert_ne!(new_clb, core::ptr::null_mut(), "Failed to allocate memory for CLB");
        unsafe { new_clb.write_bytes(0, Self::CLB_LAYOUT.size()) };
        port.clb.write(new_clb as u32);
        port.clbu.write(0);

        let new_fb = unsafe { alloc(Self::FB_LAYOUT) };
        assert_ne!(new_fb, core::ptr::null_mut(), "Failed to allocate memory for FB");
        unsafe { new_fb.write_bytes(0, Self::FB_LAYOUT.size()) };
        port.fb.write(new_fb as u32);
        port.fbu.write(0);

        for cmd_header in _self.cmd_list_mut() {
            cmd_header.prdtl.write(PRDT_LEN);
            let cmd_table = unsafe { alloc(Self::CMD_TABLE_LAYOUT) };
            assert_ne!(cmd_table, core::ptr::null_mut(), "Failed to allocate memory for CMD_TABLE");
            unsafe { cmd_table.write_bytes(0, Self::CMD_TABLE_LAYOUT.size()) };
            cmd_header.ctba.write(cmd_table as u32);
            cmd_header.ctbau.write(0);
        }

        _self.start_commands();

        let port = unsafe { _self.port_mut() };
        port.ie.write(PXIE_DHRE | PXIE_PSE | PXIE_DSE | PXIE_TFEE);

        _self
    }

    #[inline]
    unsafe fn port(&self) -> &HbaPort {
        unsafe { &*self.port_ptr }
    }

    #[inline]
    unsafe fn port_mut(&mut self) -> &mut HbaPort {
        unsafe { &mut *self.port_ptr }
    }

    #[inline]
    fn cmd_list(&self) -> &[HbaCmdHeader] {
        let port = unsafe { self.port() };
        unsafe {
            slice::from_raw_parts(
                port.clb.read() as *mut HbaCmdHeader,
                Self::CLB_LAYOUT.size() / size_of::<HbaCmdHeader>(),
            )
        }
    }

    #[inline]
    fn cmd_list_mut(&mut self) -> &mut [HbaCmdHeader] {
        let port = unsafe { self.port() };
        unsafe {
            slice::from_raw_parts_mut(
                port.clb.read() as *mut HbaCmdHeader,
                Self::CLB_LAYOUT.size() / size_of::<HbaCmdHeader>(),
            )
        }
    }

    fn start_commands(&mut self) {
        let port = unsafe { self.port_mut() };
        while (port.cmd.read() & PXCMD_CR) > 0 {}
        let mut cmd = port.cmd.read();
        cmd |= PXCMD_FRE;
        port.cmd.write(cmd);
        cmd |= PXCMD_ST;
        port.cmd.write(cmd);
    }

    fn stop_commands(&mut self) {
        let port = unsafe { self.port_mut() };
        let cmd = port.cmd.read();
        port.cmd.write(cmd & !(PXCMD_ST | PXCMD_FRE));
        while (port.cmd.read() & PXCMD_FR) > 0 || (port.cmd.read() & PXCMD_CR) > 0 {}
    }

    fn find_free_slot(&self) -> Option<usize> {
        let port = unsafe { self.port() };
        let busy = port.ci.read() | port.sact.read();
        (0..32).find(|i| busy & (1 << i) == 0)
    }

    #[allow(clippy::too_many_arguments)]
    fn issue_and_wait(
        &mut self,
        slot: usize,
        command: u8,
        lba: u64,
        block_count: u16,
        buf: *mut u8,
        buf_len: u32,
        write: bool,
    ) -> Result<(), AhciDiskError> {
        let prdt_count = buf_len.div_ceil(PRDT_MAX_DATA_BYTES) as u16;

        if prdt_count > PRDT_LEN {
            return Err(AhciDiskError::BufferTooBig)
        }

        let command_header = &mut self.cmd_list_mut()[slot];
        command_header.prdtl.write(prdt_count);
        let mut flags = HbaCmdHeaderFlags::from_bits(command_header.flags.read());
        flags.cfl = size_of::<FisRegH2D>() as u8 / size_of::<u32>() as u8;
        flags.w = write;
        command_header.flags.write(flags.to_bits());

        let command_table = unsafe {
            &mut *(command_header.ctba.read() as *mut HbaCmdTable)
        };
        unsafe { core::ptr::write_bytes(command_table, 0, 1) };

        let mut buf_addr = buf as u32;
        let mut buf_len = buf_len;
        for i in 0..(prdt_count - 1) {
            let prdt = &mut command_table.prdt.get_mut()[i as usize];
            prdt.dba.write(buf_addr);
            prdt.write_dbc(PRDT_MAX_DATA_BYTES - 1);
            prdt.write_i(true);
            buf_addr += PRDT_MAX_DATA_BYTES;
            buf_len -= PRDT_MAX_DATA_BYTES;
        }
        if buf_len > 0 {
            let prdt = &mut command_table.prdt.get_mut()[(prdt_count - 1) as usize];
            prdt.dba.write(buf_addr);
            prdt.write_dbc(buf_len - 1);
            prdt.write_i(true);
        }

        let mut command_fis = FisRegH2D {
            fis_type: FIS_TYPE_REG_H2D,
            command,
            device: 1 << 6,
            ..Default::default()
        };
        command_fis.set_c(true);
        command_fis.set_lba(lba);
        command_fis.set_count(block_count);
        let mut cfis = [0u8; 64];
        cfis[0..size_of::<FisRegH2D>()].copy_from_slice(unsafe {
            slice::from_raw_parts(
                &command_fis as *const FisRegH2D as *const u8,
                size_of::<FisRegH2D>(),
            )
        });
        command_table.cfis.write(cfis);

        let task = AhciTask {
            controller: self.controller,
            port: self.port_index,
            slot,
        };

        AHCI_TASK_MANAGER.get().borrow_mut().register_task(task);

        let port = unsafe { self.port_mut() };
        port.ci.write(port.ci.read() | (1 << slot));

        let status = loop {
            if let Some(status) = AHCI_TASK_MANAGER.get().borrow().poll(&task) &&
                status != AhciTaskStatus::Pending
            {
                break status;
            }
            unsafe { x86::halt() };
        };
        AHCI_TASK_MANAGER.get().borrow_mut().remove_task(&task);
        match status {
            AhciTaskStatus::Success => Ok(()),
            _ => Err(AhciDiskError::TaskFailed),
        }
    }

    fn ensure_identified(&mut self) -> Result<&DiskInfo, AhciDiskError> {
        match &self.info {
            Some(info) => Ok(info),
            None => {
                let slot = self.find_free_slot().ok_or(AhciDiskError::NoFreeSlot)?;
                let mut buf = [0u8; 512];
                self.issue_and_wait(
                    slot,
                    ATA_CMD_IDENTIFY,
                    0,
                    0,
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                    false
                )?;

                let block_count = u64::from_le_bytes(buf[200..208].try_into().unwrap());
                let block_size = if buf[213] & (1 << 4) != 0 {
                    u32::from_le_bytes(buf[234..238].try_into().unwrap())
                } else {
                    Self::DEFAULT_BLOCK_SIZE
                };
                let mut model: [u8; 40] = buf[54..94].try_into().unwrap();
                for i in 0..20 {
                    model.swap(i * 2, i * 2 + 1);
                }

                self.info = Some(DiskInfo {
                    block_count,
                    block_size,
                    model
                });
                Ok(self.info.as_ref().unwrap())
            }
        }
    }
}

impl fmt::Display for AhciDisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AHCI Disk on port {:x}", self.port_index)
    }
}

impl hal::disk::Disk for AhciDisk {
    fn read_blocks(&mut self, lba: u64, count: u32, buf: &mut [u8]) -> anyhow::Result<()> {
        let block_size = self.ensure_identified()?.block_size;
        let required_bytes = count as usize * block_size as usize;
        if buf.len() < required_bytes {
            return Err(AhciDiskError::BufferTooSmall.into());
        }
        let slot = self.find_free_slot().ok_or(AhciDiskError::NoFreeSlot)?;
        self.issue_and_wait(
            slot,
            ATA_CMD_READ_DMA_EX,
            lba,
            count as u16,
            buf.as_mut_ptr(),
            required_bytes as u32,
            false
        ).map_err(Into::into)
    }

    fn write_blocks(&mut self, lba: u64, count: u32, buf: &[u8]) -> anyhow::Result<()> {
        let block_size = self.ensure_identified()?.block_size;
        let required_bytes = count as usize * block_size as usize;
        if buf.len() < required_bytes {
            return Err(AhciDiskError::BufferTooSmall.into());
        }
        let slot = self.find_free_slot().ok_or(AhciDiskError::NoFreeSlot)?;
        self.issue_and_wait(
            slot,
            ATA_CMD_WRITE_DMA_EX,
            lba,
            count as u16,
            buf.as_ptr() as *mut u8,
            required_bytes as u32,
            false
        ).map_err(Into::into)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        let slot = self.find_free_slot().ok_or(AhciDiskError::NoFreeSlot)?;
        self.issue_and_wait(
            slot,
            ATA_CMD_FLUSH_EX,
            0,
            0,
            core::ptr::null_mut(),
            0,
            false
        ).map_err(Into::into)
    }

    fn block_count(&mut self) -> anyhow::Result<u64> {
        Ok(self.ensure_identified()?.block_count)
    }

    fn block_size(&mut self) -> anyhow::Result<u32> {
        Ok(self.ensure_identified()?.block_size)
    }

    fn model(&mut self) -> anyhow::Result<&str> {
        let model_bytes = &self.ensure_identified()?.model;
        let model_str = core::str::from_utf8(model_bytes)?;
        Ok(model_str.trim_end_matches('\0'))
    }
}

pub struct AhciController {
    endpoint: pci::Endpoint,
    id: AhciControllerIdentifier,
    hba: *mut HbaMem,
    interrupt_line: u8,
    disks: [Option<AhciDisk>; HBA_PORTS_COUNT as usize],
}

impl AhciController {
    pub fn new(mut endpoint: pci::Endpoint) -> Self {
        let (hba, interrupt_line) = match endpoint.extended_header {
            pci::ExtendedHeader::GeneralDevice(
                pci::GeneralDeviceHeader { bar5, interrupt_line, .. }
            ) => {
                (unsafe { &mut *(bar5 as *mut HbaMem) }, interrupt_line)
            }
        };

        let id = AhciControllerIdentifier::new(&endpoint);

        /* Enable interrupts, DMA, and memory space access */
        let mut command = endpoint.get_command();
        command.remove(pci::Command::INTERRUPT_DISABLE);
        command.insert(pci::Command::MEMORY_SPACE);
        command.insert(pci::Command::BUS_MASTER);
        endpoint.set_command(command);

        /* Enable AHCI mode and interrupts */
        let ghc = hba.ghc.read();
        hba.ghc.write(ghc | GHC_AE | GHC_IE);

        AHCI_TASK_MANAGER.get().borrow_mut().register_controller(id, hba);
        register_irq(interrupt_line, ahci_irq_handler);

        Self {
            endpoint,
            id,
            hba,
            interrupt_line,
            disks: [const { None }; HBA_PORTS_COUNT as usize],
        }
    }

    fn active_ports(&self) -> impl Iterator<Item = usize> {
        (0u32..32)
            .filter(|&i| self.hba().pi.read() & (1 << i) != 0 && self.is_port_active(i))
            .map(|i| i as usize)
    }

    fn is_port_active(&self, n: u32) -> bool {
        let port = self.get_port(n);
        let ipm = (port.ssts.read() >> 8) & 0x0F;
        let det = port.ssts.read() & 0x0F;
        det == DET_PRESENT && ipm == IPM_ACTIVE
    }

    fn get_port(&self, n: u32) -> &HbaPort {
        &self.hba().ports.get()[n as usize]
    }

    fn hba(&self) -> &HbaMem {
        unsafe { &*self.hba }
    }

    fn hba_mut(&mut self) -> &mut HbaMem {
        unsafe { &mut *self.hba }
    }
}

impl fmt::Display for AhciController {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "AHCI Disk Controller at {:x}:{:x}:{:x}:{:x}",
            self.endpoint.segment_group,
            self.endpoint.bus, 
            self.endpoint.device, 
            self.endpoint.func
        )
    }
}

impl hal::disk::DiskController for AhciController {
    type Disk = AhciDisk;

    fn is_disk_active(&self, index: usize) -> bool {
        self.is_port_active(index as u32)
    }

    fn active_disks(&self) -> Box<[usize]> {
        self.active_ports().collect()
    }

    fn get_disk(&mut self, index: usize) -> Option<&mut Self::Disk> {
        if self.disks[index].is_none() && self.is_disk_active(index) {
            self.disks[index] = Some(AhciDisk::new(
                self.id,
                index as u32,
                &mut self.hba_mut().ports.get_mut()[index] as *mut HbaPort
            ));
        }
        self.disks[index].as_mut()
    }
}
