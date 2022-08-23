#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(type_name_of_val)]

mod common;

use core::panic::PanicInfo;
use rust_os::{exit_qemu, gdt, serial_println, QemuExitCode};
use spin::Lazy;
use volatile::Volatile;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

static TEST_IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    unsafe {
        idt.double_fault
            .set_handler_fn(test_double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt
});

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

fn init_test_idt() {
    TEST_IDT.load();
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();
    Volatile::new(0).read();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    common::print_test_name(stack_overflow);

    gdt::init();
    init_test_idt();

    stack_overflow();

    panic!("[Execution continued after stack overflow]");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_os::test_panic_handler(info)
}
