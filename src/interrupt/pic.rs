use x86::io::{inb, outb};
use crate::interrupt::wait::io_wait;

const PIC1        : u16 = 0x20;
const PIC1_COMMAND: u16 = PIC1;
const PIC1_DATA   : u16 = PIC1 + 1;

const PIC2        : u16 = 0xA0;
const PIC2_COMMAND: u16 = PIC2;
const PIC2_DATA   : u16 = PIC2 + 1;

const PIC_EOI: u8 = 0x20;

const ICW1_ICW4     : u8 = 0x01;    /* Indicates that ICW4 will be present */
const ICW1_SINGLE   : u8 = 0x02;    /* Single (cascade) mode */
const ICW1_INTERVAL4: u8 = 0x04;    /* Call address interval 4 (8) */
const ICW1_LEVEL    : u8 = 0x08;    /* Level triggered (edge) mode */
const ICW1_INIT     : u8 = 0x10;    /* Initialization - required! */

const ICW4_8086      : u8 = 0x01;    /* 8086/88 (MCS-80/85) mode */
const ICW4_AUTO      : u8 = 0x02;    /* Auto (normal) EOI */
const ICW4_BUF_SLAVE : u8 = 0x08;    /* Buffered mode/slave */
const ICW4_BUF_MASTER: u8 = 0x0C;    /* Buffered mode/master */
const ICW4_SFNM      : u8 = 0x10;    /* Special fully nested (not) */

const CASCADE_IRQ: u8 = 2;

// https://wiki.osdev.org/8259_PIC#Programming_the_PIC_chips
pub fn init_pic(offset: u8) {
    let offset1 = offset;
    let offset2 = offset + 8;

    unsafe {
        outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);  // starts the initialization sequence (in cascade mode)
        io_wait();
        outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();
        outb(PIC1_DATA, offset1);                 // ICW2: Master PIC vector offset
        io_wait();
        outb(PIC2_DATA, offset2);                 // ICW2: Slave PIC vector offset
        io_wait();
        outb(PIC1_DATA, 1 << CASCADE_IRQ);    // ICW3: tell Master PIC that there is a slave PIC at IRQ2
        io_wait();
        outb(PIC2_DATA, CASCADE_IRQ);             // ICW3: tell Slave PIC its cascade identity
        io_wait();

        outb(PIC1_DATA, ICW4_8086);               // ICW4: have the PICs use 8086 mode (and not 8080 mode)
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();

        // Mask both PICs.
        outb(PIC1_DATA, 0xff);
        outb(PIC2_DATA, 0xff);
    }
}

pub fn irq_end(irq: u8) {
    if irq >= 8 {
        unsafe { outb(PIC2_COMMAND, PIC_EOI) };
    }
    unsafe { outb(PIC1_COMMAND, PIC_EOI) };
}

pub fn set_irq_mask(mut line: u8) {
    let port = if line < 8 {
        PIC1_DATA
    } else {
        line -= 8;
        PIC2_DATA
    };

    unsafe {
        let mask = inb(port) | (1 << line);
        outb(port, mask);
    }
}

pub fn clear_irq_mask(mut line: u8) {
    let port = if line < 8 {
        PIC1_DATA
    } else {
        line -= 8;
        PIC2_DATA
    };

    unsafe {
        let mask = inb(port) & !(1 << line);
        outb(port, mask);
    }
}
