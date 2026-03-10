// === PRIVATE API ===

const DEFAULT_VGA_WIDTH: usize = 80;
const DEFAULT_VGA_HEIGHT: usize = 25;

#[derive(Debug, Clone, Copy)]
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
}

#[derive(Debug, Clone, Copy)]
pub struct VgaStyle {
    pub foreground: VgaColor,
    pub background: VgaColor,
    pub blink: bool,
}

impl Default for VgaStyle {
    fn default() -> Self {
        Self {
            foreground: VgaColor::White,
            background: VgaColor::Black,
            blink: false,
        }
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

// === PUBLIC API ===

pub struct Vga<'a> {
    buffer: &'a mut [u16],
    width: usize,
    height: usize,
    cursor_x: usize,
    cursor_y: usize,
    style: VgaStyle,
}

impl<'a> Vga<'a> {
    pub fn new(buffer: &'a mut [u16], width: usize, height: usize) -> Vga<'a> {
        Self {
            buffer,
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            style: VgaStyle::default(),
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
                c => {
                    self.plot(
                        VgaChar { c, style: self.style },
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
        for y in 0..self.height {
            for x in 0..self.width {
                self.buffer[y * self.width + x] = 0;
            }
        }
    }
}

impl Default for Vga<'static> {
    fn default() -> Vga<'static> {
        Self::new(
            // SAFETY: mono-threaded kernel
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

impl core::fmt::Write for Vga<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.puts(s);
        Ok(())
    }
}
