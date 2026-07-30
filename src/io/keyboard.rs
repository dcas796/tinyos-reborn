use core::sync::atomic::{AtomicBool, Ordering};
use x86::io::inb;
use crate::logln;

pub const KEYBOARD_IRQ: u8 = 1;

const DATA_PORT: u16 = 0x60;
const STATUS_COMMAND_REG: u16 = 0x64;
const EXTENDED_CODE: u8 = 0xe0;

// TODO: Dynamically detect scan code set
const SCAN_CODE_SET: ScanCodeSet = ScanCodeSet::One;

pub fn __interrupt() {
    static EXTENDED: AtomicBool = AtomicBool::new(false);

    let status = Status::from_bits(unsafe { inb(STATUS_COMMAND_REG) });

    if !status.time_out_error &&
        !status.parity_error &&
        status.output_buffer_status == BufferStatus::Full {
        let byte = unsafe { inb(DATA_PORT) };

        if byte == EXTENDED_CODE {
            EXTENDED.store(true, Ordering::Relaxed);
        } else {
            let scan_code = ScanCode::from_byte(
                EXTENDED.load(Ordering::Relaxed),
                byte,
            );
            EXTENDED.store(false, Ordering::Relaxed);

            if let Some(physical_key) = scan_code.physical_key(SCAN_CODE_SET) {
                logln!("key: {physical_key:?}, is_down: {}", scan_code.is_down());
            } else {
                logln!("Unrecognized scan code: {}{byte:#x}", if scan_code.extended { "0xe0 " } else { "" });
            }
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BufferStatus {
    Empty = 0,
    Full  = 1,
}

impl BufferStatus {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 1 {
            0 => Self::Empty,
            1 => Self::Full,
            _ => unreachable!(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum DataDestination {
    ToDevice     = 0,
    ToController = 1,
}

impl DataDestination {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 1 {
            0 => Self::ToDevice,
            1 => Self::ToController,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Status {
    output_buffer_status: BufferStatus,
    input_buffer_status: BufferStatus,
    data_destination: DataDestination,
    time_out_error: bool,
    parity_error: bool,
}

impl Status {
    fn from_bits(bits: u8) -> Self {
        Self {
            output_buffer_status: BufferStatus::from_bits(bits & 1),
            input_buffer_status: BufferStatus::from_bits((bits >> 1) & 1),
            data_destination: DataDestination::from_bits((bits >> 3) & 1),
            time_out_error: (bits >> 6) & 1 == 1,
            parity_error: (bits >> 7) & 1 == 1,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PhysicalKey {
    Q, W, E, R, T, Y, U, I, O, P,
    A, S, D, F, G, H, J, K, L,
    Z, X, C, V, B, N, M,
    Zero, One, Two, Three, Four, Five, Six, Seven, Eight, Nine,
    SemiColon, Tick, Comma, Dot, Slash,
    BackTick, Minus, Equals, Backslash, Backspace, Space, Tab, CapsLock,
    LeftShift, LeftControl, LeftGui, LeftAlt,
    RightShift, RightControl, RightGui, RightAlt, Apps,
    Enter, Esc,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    /*PrintScreen,*/ Scroll, /*Pause,*/
    Insert, Home, PageUp, Delete, End, PageDown, UpArrow, LeftArrow, DownArrow, RightArrow,
    NumLock, OpenSquareBracket, CloseSquareBracket,
    KeypadSlash, KeypadStar, KeypadMinus, KeypadPlus, KeypadEnter, KeypadDot, KeypadZero,
    KeypadOne, KeypadTwo, KeypadThree, KeypadFour, KeypadFive, KeypadSix, KeypadSeven, KeypadEight,
    KeypadNine,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ScanCodeSet {
    One,
    Two,
    Three,
}

#[derive(Debug, Copy, Clone)]
struct ScanCode {
    extended: bool,
    code: u8,
}

impl ScanCode {
    fn from_byte(extended: bool, byte: u8) -> Self {
        Self {
            extended,
            code: byte,
        }
    }

    fn make_code(&self) -> u8 {
        self.code & 0x7f
    }

    fn is_down(&self) -> bool {
        self.code & 0x80 == 0
    }

    fn physical_key(&self, scan_code_set: ScanCodeSet) -> Option<PhysicalKey> {
        match scan_code_set {
            ScanCodeSet::One => self._physical_key_scan_code_set_one(),
            ScanCodeSet::Two => todo!(),
            ScanCodeSet::Three => todo!(),
        }
    }

    fn _physical_key_scan_code_set_one(&self) -> Option<PhysicalKey> {
        match (self.extended, self.make_code()) {
            // Letters
            (false, 0x1e) => Some(PhysicalKey::A),
            (false, 0x30) => Some(PhysicalKey::B),
            (false, 0x2e) => Some(PhysicalKey::C),
            (false, 0x20) => Some(PhysicalKey::D),
            (false, 0x12) => Some(PhysicalKey::E),
            (false, 0x21) => Some(PhysicalKey::F),
            (false, 0x22) => Some(PhysicalKey::G),
            (false, 0x23) => Some(PhysicalKey::H),
            (false, 0x17) => Some(PhysicalKey::I),
            (false, 0x24) => Some(PhysicalKey::J),
            (false, 0x25) => Some(PhysicalKey::K),
            (false, 0x26) => Some(PhysicalKey::L),
            (false, 0x32) => Some(PhysicalKey::M),
            (false, 0x31) => Some(PhysicalKey::N),
            (false, 0x18) => Some(PhysicalKey::O),
            (false, 0x19) => Some(PhysicalKey::P),
            (false, 0x10) => Some(PhysicalKey::Q),
            (false, 0x13) => Some(PhysicalKey::R),
            (false, 0x1f) => Some(PhysicalKey::S),
            (false, 0x14) => Some(PhysicalKey::T),
            (false, 0x16) => Some(PhysicalKey::U),
            (false, 0x2f) => Some(PhysicalKey::V),
            (false, 0x11) => Some(PhysicalKey::W),
            (false, 0x2d) => Some(PhysicalKey::X),
            (false, 0x15) => Some(PhysicalKey::Y),
            (false, 0x2c) => Some(PhysicalKey::Z),

            // Numbers
            (false, 0x0b) => Some(PhysicalKey::Zero),
            (false, 0x02) => Some(PhysicalKey::One),
            (false, 0x03) => Some(PhysicalKey::Two),
            (false, 0x04) => Some(PhysicalKey::Three),
            (false, 0x05) => Some(PhysicalKey::Four),
            (false, 0x06) => Some(PhysicalKey::Five),
            (false, 0x07) => Some(PhysicalKey::Six),
            (false, 0x08) => Some(PhysicalKey::Seven),
            (false, 0x09) => Some(PhysicalKey::Eight),
            (false, 0x0a) => Some(PhysicalKey::Nine),

            // Punctuation / symbols
            (false, 0x27) => Some(PhysicalKey::SemiColon),
            (false, 0x28) => Some(PhysicalKey::Tick),
            (false, 0x33) => Some(PhysicalKey::Comma),
            (false, 0x34) => Some(PhysicalKey::Dot),
            (false, 0x35) => Some(PhysicalKey::Slash),
            (false, 0x29) => Some(PhysicalKey::BackTick),
            (false, 0x0c) => Some(PhysicalKey::Minus),
            (false, 0x0d) => Some(PhysicalKey::Equals),
            (false, 0x2b) => Some(PhysicalKey::Backslash),
            (false, 0x1a) => Some(PhysicalKey::OpenSquareBracket),
            (false, 0x1b) => Some(PhysicalKey::CloseSquareBracket),

            // Editing / whitespace
            (false, 0x0e) => Some(PhysicalKey::Backspace),
            (false, 0x39) => Some(PhysicalKey::Space),
            (false, 0x0f) => Some(PhysicalKey::Tab),
            (false, 0x3a) => Some(PhysicalKey::CapsLock),
            (false, 0x1c) => Some(PhysicalKey::Enter),
            (false, 0x01) => Some(PhysicalKey::Esc),

            // Modifiers
            (false, 0x2a) => Some(PhysicalKey::LeftShift),
            (false, 0x1d) => Some(PhysicalKey::LeftControl),
            (false, 0x38) => Some(PhysicalKey::LeftAlt),
            (false, 0x36) => Some(PhysicalKey::RightShift),
            (true, 0x5b) => Some(PhysicalKey::LeftGui),
            (true, 0x1d) => Some(PhysicalKey::RightControl),
            (true, 0x5c) => Some(PhysicalKey::RightGui),
            (true, 0x38) => Some(PhysicalKey::RightAlt),
            (true, 0x5d) => Some(PhysicalKey::Apps),

            // Function keys
            (false, 0x3b) => Some(PhysicalKey::F1),
            (false, 0x3c) => Some(PhysicalKey::F2),
            (false, 0x3d) => Some(PhysicalKey::F3),
            (false, 0x3e) => Some(PhysicalKey::F4),
            (false, 0x3f) => Some(PhysicalKey::F5),
            (false, 0x40) => Some(PhysicalKey::F6),
            (false, 0x41) => Some(PhysicalKey::F7),
            (false, 0x42) => Some(PhysicalKey::F8),
            (false, 0x43) => Some(PhysicalKey::F9),
            (false, 0x44) => Some(PhysicalKey::F10),
            (false, 0x57) => Some(PhysicalKey::F11),
            (false, 0x58) => Some(PhysicalKey::F12),

            // PrintScreen: two make bytes E0,2A E0,37 map to the same key
            // (true, 0x2a) => Some(PhysicalKey::PrintScreen),
            // (true, 0x37) => Some(PhysicalKey::PrintScreen),

            (false, 0x46) => Some(PhysicalKey::Scroll),
            // Pause (E1,1D,45 / E1,9D,C5) still can't be represented here —
            // it needs special-casing upstream before reaching ScanCode.

            // Navigation cluster
            (true, 0x52) => Some(PhysicalKey::Insert),
            (true, 0x47) => Some(PhysicalKey::Home),
            (true, 0x49) => Some(PhysicalKey::PageUp),
            (true, 0x53) => Some(PhysicalKey::Delete),
            (true, 0x4f) => Some(PhysicalKey::End),
            (true, 0x51) => Some(PhysicalKey::PageDown),
            (true, 0x48) => Some(PhysicalKey::UpArrow),
            (true, 0x4b) => Some(PhysicalKey::LeftArrow),
            (true, 0x50) => Some(PhysicalKey::DownArrow),
            (true, 0x4d) => Some(PhysicalKey::RightArrow),

            // Keypad
            (false, 0x45) => Some(PhysicalKey::NumLock),
            (true, 0x35)  => Some(PhysicalKey::KeypadSlash),
            (false, 0x37) => Some(PhysicalKey::KeypadStar),
            (false, 0x4a) => Some(PhysicalKey::KeypadMinus),
            (false, 0x4e) => Some(PhysicalKey::KeypadPlus),
            (true, 0x1c)  => Some(PhysicalKey::KeypadEnter),
            (false, 0x53) => Some(PhysicalKey::KeypadDot),
            (false, 0x52) => Some(PhysicalKey::KeypadZero),
            (false, 0x4f) => Some(PhysicalKey::KeypadOne),
            (false, 0x50) => Some(PhysicalKey::KeypadTwo),
            (false, 0x51) => Some(PhysicalKey::KeypadThree),
            (false, 0x4b) => Some(PhysicalKey::KeypadFour),
            (false, 0x4c) => Some(PhysicalKey::KeypadFive),
            (false, 0x4d) => Some(PhysicalKey::KeypadSix),
            (false, 0x47) => Some(PhysicalKey::KeypadSeven),
            (false, 0x48) => Some(PhysicalKey::KeypadEight),
            (false, 0x49) => Some(PhysicalKey::KeypadNine),

            _ => None,
        }
    }
}
