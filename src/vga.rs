use core::cell::RefCell;
use core::fmt::{Display, Formatter, Write};
use int_enum::IntEnum;
use crate::util::interrupt_lock::{InterruptLock, InterruptLockRef};
use crate::vga::VgaColor::{Black, White};

const DEFAULT_VGA_WIDTH: usize = 80;
const DEFAULT_VGA_HEIGHT: usize = 25;

static VGA: InterruptLock<RefCell<Option<Vga>>> = InterruptLock::new(RefCell::new(None));

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        if let Some(vga) = $crate::vga::vga().borrow_mut().as_mut() {
            use core::fmt::Write;
            _ = core::write!(vga, $($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        print!("\n");
    }};
}

pub fn init_vga() {
    let mut vga = Vga::default();
    vga.clear_screen();
    vga.set_foreground(White);
    vga.set_background(Black);
    *VGA.get().borrow_mut() = Some(vga);
}

pub fn vga<'a>() -> InterruptLockRef<'a, RefCell<Option<Vga<'static>>>> {
    VGA.get()
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, IntEnum)]
pub enum VgaColor {
    Black        = 0x0,
    Blue         = 0x1,
    Green        = 0x2,
    Cyan         = 0x3,
    Red          = 0x4,
    Magenta      = 0x5,
    Brown        = 0x6,
    White        = 0x7,
    Gray         = 0x8,
    LightBlue    = 0x9,
    LightGreen   = 0xA,
    LightCyan    = 0xB,
    LightRed     = 0xC,
    LightMagenta = 0xD,
    Yellow       = 0xE,
    BrightWhite  = 0xF,
    End          = 0xFF,
}

impl Display for VgaColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_char(0x11 as char)?;
        f.write_char(*self as u8 as char)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VgaStyle {
    pub foreground: VgaColor,
    pub background: VgaColor,
    pub blink: bool,
}

impl VgaStyle {
    pub const fn new(foreground: VgaColor, background: VgaColor, blink: bool) -> Self {
        Self { foreground, background, blink }
    }
}

impl Default for VgaStyle {
    fn default() -> Self {
        Self::new(VgaColor::White, VgaColor::Black, false)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VgaChar {
    pub c: char,
    pub style: VgaStyle,
}

impl From<VgaChar> for u16 {
    fn from(char: VgaChar) -> u16 {
        let mut word = 0u16;
        word |= char.style.background as u16;
        word <<= 4;
        word |= char.style.foreground as u16;
        word <<= 8;
        word |= char.c as u16;
        if char.style.blink {
            word |= 0x7000u16;
        } else {
            word &= 0x7fffu16;
        }
        word
    }
}

pub struct Vga<'a> {
    buffer: &'a mut [u16],
    width: usize,
    height: usize,
    cursor_x: usize,
    cursor_y: usize,
    current_style: VgaStyle,
    default_style: VgaStyle,
    is_styling: bool,
}

impl<'a> Vga<'a> {
    pub fn new(buffer: &'a mut [u16], width: usize, height: usize) -> Vga<'a> {
        Self {
            buffer,
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            current_style: VgaStyle::default(),
            default_style: VgaStyle::default(),
            is_styling: false,
        }
    }
}

impl Vga<'_> {
    pub fn plot(&mut self, char: VgaChar, x: usize, y: usize) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            return Err("Out of bounds");
        }

        self.buffer[y * self.width + x] = char.into();

        Ok(())
    }

    pub fn puts(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '\n' => self.newline(),
                '\x11' => self.is_styling = true,
                c if self.is_styling => {
                    match VgaColor::try_from(c as u8) {
                        Ok(VgaColor::End) => self.current_style = self.default_style,
                        Ok(color) => self.current_style.foreground = color,
                        Err(_) => {},
                    }
                    self.is_styling = false;
                }
                c => {
                    self.plot(
                        VgaChar { c, style: self.current_style },
                        self.cursor_x,
                        self.cursor_y
                    ).expect("Failed to plot string");
                    self.step_cursor();
                }
            }
        }
    }

    pub fn step_cursor(&mut self) {
        self.cursor_x += 1;

        if self.cursor_x >= self.width {
            self.newline();
        }
    }

    pub fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;

        if self.cursor_y >= self.height {
            self.cursor_y = 0;
            self.clear_screen();
        }
    }

    pub fn clear_screen(&mut self) {
        self.cursor_x = 0;
        self.cursor_y = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                self.buffer[y * self.width + x] = 0;
            }
        }
    }

    pub fn style(&self) -> VgaStyle {
        self.default_style
    }

    pub fn set_foreground(&mut self, color: VgaColor) {
        if color == VgaColor::End { return }
        self.default_style.foreground = color;
        self.current_style = self.default_style;
    }

    pub fn set_background(&mut self, color: VgaColor) {
        if color == VgaColor::End { return }
        self.default_style.background = color;
        self.current_style = self.default_style;
    }
}

impl<'a> Default for Vga<'a> {
    fn default() -> Vga<'a> {
        Self::new(
            unsafe {
                core::slice::from_raw_parts_mut(
                    0xB8000 as *mut u16,
                    DEFAULT_VGA_WIDTH * DEFAULT_VGA_HEIGHT
                )
            },
            DEFAULT_VGA_WIDTH,
            DEFAULT_VGA_HEIGHT,
        )
    }
}

impl Write for Vga<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.puts(s);
        Ok(())
    }
}
