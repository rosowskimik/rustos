use core::fmt::{self, Write};

use spin::{Lazy, Mutex};
use uart_16550::SerialPort;
use x86_64::instructions;

pub static SERIAL1: Lazy<Mutex<SerialPort>> = Lazy::new(|| {
    let mut serial = unsafe { SerialPort::new(0x3f8) };
    serial.init();
    Mutex::new(serial)
});

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    instructions::interrupts::without_interrupts(|| {
        SERIAL1.lock().write_fmt(args).unwrap();
    });
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

/// Prints to the host through the serial interface, followed by a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}
