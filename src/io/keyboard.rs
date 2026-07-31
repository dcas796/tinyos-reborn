use core::cell::RefCell;
use core::fmt::{Display, Formatter};
use core::sync::atomic::{AtomicBool, Ordering};
use x86::io::inb;
use crate::util::interrupt_lock::InterruptLock;

pub const KEYBOARD_IRQ: u8 = 1;

const DATA_PORT: u16 = 0x60;
const STATUS_COMMAND_REG: u16 = 0x64;
const EXTENDED_CODE: u8 = 0xe0;

#[allow(clippy::type_complexity)]
static KEYBOARD_HANDLER: InterruptLock<RefCell<Option<fn(ScanCode, &Meta)>>> = InterruptLock::new(RefCell::new(None));
pub fn set_keyboard_handler(f: fn(ScanCode, &Meta)) {
    *KEYBOARD_HANDLER.get().borrow_mut() = Some(f);
}

pub fn __interrupt() {
    static EXTENDED: AtomicBool = AtomicBool::new(false);
    static META: Meta = Meta::new();

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

            match scan_code.physical_key(ScanCodeSet::default()) {
                Some(PhysicalKey::LeftShift) | Some(PhysicalKey::RightShift) => {
                    META.shift.store(scan_code.is_down(), Ordering::Relaxed);
                }
                Some(PhysicalKey::LeftControl) | Some(PhysicalKey::RightControl) => {
                    META.control.store(scan_code.is_down(), Ordering::Relaxed);
                }
                Some(PhysicalKey::LeftAlt) | Some(PhysicalKey::RightAlt) => {
                    META.alt.store(scan_code.is_down(), Ordering::Relaxed);
                }
                Some(PhysicalKey::LeftGui) | Some(PhysicalKey::RightGui) => {
                    META.gui.store(scan_code.is_down(), Ordering::Relaxed);
                }
                Some(PhysicalKey::CapsLock) if scan_code.is_down() => {
                    META.caps_lock.fetch_not(Ordering::Relaxed);
                }
                _ => {}
            }

            if let Some(handler) = *KEYBOARD_HANDLER.get().borrow() {
                handler(scan_code, &META);
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum KeyboardLayout {
    #[default]
    EnUs,
}

impl KeyboardLayout {
    pub fn get_char(&self, physical_key: PhysicalKey, meta: &Meta) -> Option<char> {
        match self {
            Self::EnUs => Self::_get_char_en_us(physical_key, meta),
        }
    }
}

#[derive(Debug)]
pub struct Meta {
    control: AtomicBool,
    alt: AtomicBool,
    shift: AtomicBool,
    caps_lock: AtomicBool,
    gui: AtomicBool,
}

impl Meta {
    pub const fn new() -> Self {
        Self {
            control: AtomicBool::new(false),
            alt: AtomicBool::new(false),
            shift: AtomicBool::new(false),
            caps_lock: AtomicBool::new(false),
            gui: AtomicBool::new(false),
        }
    }

    pub fn control(&self) -> bool {
        self.control.load(Ordering::Relaxed)
    }

    pub fn alt(&self) -> bool {
        self.alt.load(Ordering::Relaxed)
    }

    pub fn shift(&self) -> bool {
        self.shift.load(Ordering::Relaxed)
    }

    pub fn caps_lock(&self) -> bool {
        self.caps_lock.load(Ordering::Relaxed)
    }

    pub fn gui(&self) -> bool {
        self.gui.load(Ordering::Relaxed)
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PhysicalKey {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
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

impl PhysicalKey {
    pub fn as_char(&self, layout: KeyboardLayout, meta: &Meta) -> Option<char> {
        layout.get_char(*self, meta)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum ScanCodeSet {
    #[default] // TODO: Dynamically detect scan code set
    One,
    Two,
    Three,
}

#[derive(Debug, Copy, Clone)]
pub struct ScanCode {
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

    pub fn is_down(&self) -> bool {
        self.code & 0x80 == 0
    }

    pub fn physical_key(&self, scan_code_set: ScanCodeSet) -> Option<PhysicalKey> {
        match scan_code_set {
            ScanCodeSet::One => self._physical_key_scan_code_set_one(),
            ScanCodeSet::Two => todo!(),
            ScanCodeSet::Three => todo!(),
        }
    }
}

impl Display for ScanCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.extended {
            f.write_str("0xe0 ")?;
        }
        write!(f, "{:#04x}", self.code)
    }
}

/* === MAPPINGS === */

impl KeyboardLayout {
    fn _get_char_en_us(physical_key: PhysicalKey, meta: &Meta) -> Option<char> {
        use PhysicalKey::*;
        
        if meta.control() || meta.alt() || meta.gui() {
            return None;
        }

        // Letters: single rule instead of 52 match arms.
        if let A | B | C | D | E | F | G | H | I | J | K | L | M | N | O | P | Q | R | S | T
            | U | V | W | X | Y | Z = physical_key
        {
            let lower = b'a' + (physical_key as u8 - A as u8);
            let c = lower as char;
            return Some(if meta.shift() || meta.caps_lock() { c.to_ascii_uppercase() } else { c });
        }

        // Everything else: one (unshifted, shifted) pair per key.
        let (lower, upper) = match physical_key {
            Zero => ('0', ')'),
            One => ('1', '!'),
            Two => ('2', '@'),
            Three => ('3', '#'),
            Four => ('4', '$'),
            Five => ('5', '%'),
            Six => ('6', '^'),
            Seven => ('7', '&'),
            Eight => ('8', '*'),
            Nine => ('9', '('),

            SemiColon => (';', ':'),
            Tick => ('\'', '"'),
            Comma => (',', '<'),
            Dot => ('.', '>'),
            Slash => ('/', '?'),
            BackTick => ('`', '~'),
            Minus => ('-', '_'),
            Equals => ('=', '+'),
            Backslash => ('\\', '|'),
            OpenSquareBracket => ('[', '{'),
            CloseSquareBracket => (']', '}'),

            Space => (' ', ' '),
            Tab => ('\t', '\t'),
            Enter | KeypadEnter => ('\n', '\n'),

            KeypadZero => ('0', '0'),
            KeypadOne => ('1', '1'),
            KeypadTwo => ('2', '2'),
            KeypadThree => ('3', '3'),
            KeypadFour => ('4', '4'),
            KeypadFive => ('5', '5'),
            KeypadSix => ('6', '6'),
            KeypadSeven => ('7', '7'),
            KeypadEight => ('8', '8'),
            KeypadNine => ('9', '9'),
            KeypadDot => ('.', '.'),
            KeypadSlash => ('/', '/'),
            KeypadStar => ('*', '*'),
            KeypadMinus => ('-', '-'),
            KeypadPlus => ('+', '+'),

            _ => return None,
        };

        Some(if meta.shift() { upper } else { lower })
    }
}

impl ScanCode {
    fn _physical_key_scan_code_set_one(&self) -> Option<PhysicalKey> {
        use PhysicalKey::*;
        
        match (self.extended, self.make_code()) {
            // Letters
            (false, 0x1e) => Some(A),
            (false, 0x30) => Some(B),
            (false, 0x2e) => Some(C),
            (false, 0x20) => Some(D),
            (false, 0x12) => Some(E),
            (false, 0x21) => Some(F),
            (false, 0x22) => Some(G),
            (false, 0x23) => Some(H),
            (false, 0x17) => Some(I),
            (false, 0x24) => Some(J),
            (false, 0x25) => Some(K),
            (false, 0x26) => Some(L),
            (false, 0x32) => Some(M),
            (false, 0x31) => Some(N),
            (false, 0x18) => Some(O),
            (false, 0x19) => Some(P),
            (false, 0x10) => Some(Q),
            (false, 0x13) => Some(R),
            (false, 0x1f) => Some(S),
            (false, 0x14) => Some(T),
            (false, 0x16) => Some(U),
            (false, 0x2f) => Some(V),
            (false, 0x11) => Some(W),
            (false, 0x2d) => Some(X),
            (false, 0x15) => Some(Y),
            (false, 0x2c) => Some(Z),

            // Numbers
            (false, 0x0b) => Some(Zero),
            (false, 0x02) => Some(One),
            (false, 0x03) => Some(Two),
            (false, 0x04) => Some(Three),
            (false, 0x05) => Some(Four),
            (false, 0x06) => Some(Five),
            (false, 0x07) => Some(Six),
            (false, 0x08) => Some(Seven),
            (false, 0x09) => Some(Eight),
            (false, 0x0a) => Some(Nine),

            // Punctuation / symbols
            (false, 0x27) => Some(SemiColon),
            (false, 0x28) => Some(Tick),
            (false, 0x33) => Some(Comma),
            (false, 0x34) => Some(Dot),
            (false, 0x35) => Some(Slash),
            (false, 0x29) => Some(BackTick),
            (false, 0x0c) => Some(Minus),
            (false, 0x0d) => Some(Equals),
            (false, 0x2b) => Some(Backslash),
            (false, 0x1a) => Some(OpenSquareBracket),
            (false, 0x1b) => Some(CloseSquareBracket),

            // Editing / whitespace
            (false, 0x0e) => Some(Backspace),
            (false, 0x39) => Some(Space),
            (false, 0x0f) => Some(Tab),
            (false, 0x3a) => Some(CapsLock),
            (false, 0x1c) => Some(Enter),
            (false, 0x01) => Some(Esc),

            // Modifiers
            (false, 0x2a) => Some(LeftShift),
            (false, 0x1d) => Some(LeftControl),
            (false, 0x38) => Some(LeftAlt),
            (false, 0x36) => Some(RightShift),
            (true,  0x5b) => Some(LeftGui),
            (true,  0x1d) => Some(RightControl),
            (true,  0x5c) => Some(RightGui),
            (true,  0x38) => Some(RightAlt),
            (true,  0x5d) => Some(Apps),

            // Function keys
            (false, 0x3b) => Some(F1),
            (false, 0x3c) => Some(F2),
            (false, 0x3d) => Some(F3),
            (false, 0x3e) => Some(F4),
            (false, 0x3f) => Some(F5),
            (false, 0x40) => Some(F6),
            (false, 0x41) => Some(F7),
            (false, 0x42) => Some(F8),
            (false, 0x43) => Some(F9),
            (false, 0x44) => Some(F10),
            (false, 0x57) => Some(F11),
            (false, 0x58) => Some(F12),

            // PrintScreen: two make bytes E0,2A E0,37 map to the same key
            // (true, 0x2a) => Some(PrintScreen),
            // (true, 0x37) => Some(PrintScreen),

            (false, 0x46) => Some(Scroll),
            // Pause (E1,1D,45 / E1,9D,C5) still can't be represented here —
            // it needs special-casing upstream before reaching ScanCode.

            // Navigation cluster
            (true,  0x52) => Some(Insert),
            (true,  0x47) => Some(Home),
            (true,  0x49) => Some(PageUp),
            (true,  0x53) => Some(Delete),
            (true,  0x4f) => Some(End),
            (true,  0x51) => Some(PageDown),
            (true,  0x48) => Some(UpArrow),
            (true,  0x4b) => Some(LeftArrow),
            (true,  0x50) => Some(DownArrow),
            (true,  0x4d) => Some(RightArrow),

            // Keypad
            (false, 0x45) => Some(NumLock),
            (true,  0x35) => Some(KeypadSlash),
            (false, 0x37) => Some(KeypadStar),
            (false, 0x4a) => Some(KeypadMinus),
            (false, 0x4e) => Some(KeypadPlus),
            (true,  0x1c) => Some(KeypadEnter),
            (false, 0x53) => Some(KeypadDot),
            (false, 0x52) => Some(KeypadZero),
            (false, 0x4f) => Some(KeypadOne),
            (false, 0x50) => Some(KeypadTwo),
            (false, 0x51) => Some(KeypadThree),
            (false, 0x4b) => Some(KeypadFour),
            (false, 0x4c) => Some(KeypadFive),
            (false, 0x4d) => Some(KeypadSix),
            (false, 0x47) => Some(KeypadSeven),
            (false, 0x48) => Some(KeypadEight),
            (false, 0x49) => Some(KeypadNine),

            _ => None,
        }
    }
}
