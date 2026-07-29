const COMMAND: u16 = 0x43;
const SYSTEM_TIMER: u16 = 0x40;

pub fn set_timer_freq(freq: u32) -> Result<(), InvalidFrequencyError> {
    if freq > 1193182 || freq <= 18 {
        return Err(InvalidFrequencyError);
    }

    let divisor: u16 = (1193180 / freq) as u16;
    unsafe {
        x86::io::outb(COMMAND, Command {
            channel: Channel::Channel0,
            access_mode: AccessMode::LowHighByte,
            operating_mode: OperatingMode::Mode3,
            counting_mode: CountingMode::Binary,
        }.into_bits());
        x86::io::outb(SYSTEM_TIMER, (divisor & 0xFF) as u8);
        x86::io::outb(SYSTEM_TIMER, (divisor >> 8) as u8);
    }

    Ok(())
}

#[derive(Debug, Copy, Clone)]
pub struct InvalidFrequencyError;

#[repr(u8)]
enum Channel {
    Channel0 = 0b00,
    Channel1 = 0b01,
    Channel2 = 0b10,
    ReadBack = 0b11,
}

#[repr(u8)]
enum AccessMode {
    LatchCount   = 0b00,
    LowByteOnly  = 0b01,
    HighByteOnly = 0b10,
    LowHighByte  = 0b11,
}

#[repr(u8)]
enum OperatingMode {
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
enum CountingMode {
    Binary = 0b0,
    BCD    = 0b1,
}

struct Command {
    channel: Channel,
    access_mode: AccessMode,
    operating_mode: OperatingMode,
    counting_mode: CountingMode,
}

impl Command {
    fn into_bits(self) -> u8 {
        (self.channel as u8 & 0b11) << 6 |
        (self.access_mode as u8 & 0b11) << 4 |
        (self.operating_mode as u8 & 0b111) << 1 |
        (self.counting_mode as u8 & 0b1)
    }
}