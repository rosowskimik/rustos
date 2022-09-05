#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use bootloader::{entry_point, BootInfo};
use rust_os::{
    self, allocator, hlt_loop,
    memory::{self, BootInfoFrameAllocator},
    println,
};
use x86_64::VirtAddr;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    #[cfg(test)]
    test_main();
    rust_os::init();

    println!("Hello World!");

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let x = Box::new(41);
    println!("Box: {}\naddr: {:p}", x, x);

    let mut y = Vec::new();
    y.extend(0..500);
    // (0..500).for_each(|i| y.push(i));
    println!("\nVec: {:?}\naddr: {:p}", &y[..5], y.as_slice());

    let z = Rc::new(42);
    let z2 = z.clone();
    println!("\nRc ref count: {}", Rc::strong_count(&z));
    core::mem::drop(z2);
    println!("Rc ref count: {}", Rc::strong_count(&z));
    println!("Rc: {}\naddr: {:p}", z, z);

    println!("It did not crash!");
    hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_os::test_panic_handler(info)
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn trivial_assertion() {
        assert_eq!(1, 1);
    }
}
