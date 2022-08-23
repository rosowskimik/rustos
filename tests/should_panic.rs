#![no_std]
#![no_main]
#![feature(type_name_of_val)]

mod common;

use core::panic::PanicInfo;
use rust_os::{exit_qemu, serial_println, QemuExitCode};

fn should_fail() {
    assert_eq!(0, 1);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    common::print_test_name(should_fail);

    should_fail();

    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
