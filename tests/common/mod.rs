use core::any;
use rust_os::serial_print;

/// Convinience wrapper for consistent logging between tests with or without the harness.
pub fn print_test_name<T>(test: T) {
    serial_print!("{}...\t", any::type_name_of_val(&test));
}
