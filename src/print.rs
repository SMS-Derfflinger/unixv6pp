use core::fmt::{self, Write};

use crate::serial::serial_write;

struct Console;
const CONSOLE: Console = Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial_write(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn do_print(args: fmt::Arguments) {
    CONSOLE.write_fmt(args).ok();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::do_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println_warn {
    ($($arg:tt)*) => {
        $crate::println!("[kernel: warn] {}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println_debug {
    ($($arg:tt)*) => {
        $crate::println!("[kernel:debug] {}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println_info {
    ($($arg:tt)*) => {
        $crate::println!("[kernel: info] {}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println_fatal {
    () => {
        $crate::println!("[kernel:fatal] ")
    };
    ($($arg:tt)*) => {
        $crate::println!("[kernel:fatal] {}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println_trace {
    () => {
        $crate::println!("[kernel:trace]")
    };
    ($fmt:literal, $($arg:expr $(,)?)*) => {
        $crate::println!("[kernel:trace] {}", format_args!($fmt, $($arg,)*))
    };
}
