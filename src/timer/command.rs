#[repr(u8)]
pub enum Channel {
    Channel0 = 0b00,
    Channel1 = 0b01,
    Channel2 = 0b10,
    ReadBack = 0b11,
}

#[repr(u8)]
pub enum AccessMode {
    LatchCount   = 0b00,
    LowByteOnly  = 0b01,
    HighByteOnly = 0b10,
    LowHighByte  = 0b11,
}

#[repr(u8)]
pub enum OperatingMode {
    /// Interrupt on terminal count
    Mode0 = 0b000,
    /// Hardware re-triggerable one-shot
    Mode1 = 0b001,
    /// Rate generator
    Mode2 = 0b010,
    /// Square wave generator
    Mode3 = 0b011,
    /// Software triggered strobe
    Mode4 = 0b100,
    /// Hardware triggered strobe
    Mode5 = 0b101,
}

#[repr(u8)]
pub enum CountingMode {
    Binary = 0b0,
    BCD    = 0b1,
}

pub struct Command {
    pub channel: Channel,
    pub access_mode: AccessMode,
    pub operating_mode: OperatingMode,
    pub counting_mode: CountingMode,
}

impl Command {
    pub fn into_bits(self) -> u8 {
        (self.channel as u8 & 0b11) << 6 |
        (self.access_mode as u8 & 0b11) << 4 |
        (self.operating_mode as u8 & 0b111) << 1 |
        (self.counting_mode as u8 & 0b1)
    }
}