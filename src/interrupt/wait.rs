pub fn io_wait() {
    unsafe {
        x86::io::outb(0x80, 0);
    }
}