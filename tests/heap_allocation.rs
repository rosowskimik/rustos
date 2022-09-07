#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rust_os::{
    allocator, hlt_loop,
    memory::{self, BootInfoFrameAllocator},
};
use x86_64::VirtAddr;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    rust_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    test_main();
    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_os::test_panic_handler(info)
}

#[test_case]
fn simple_allocation() {
    let heap_val_1 = Box::new(1);
    let heap_val_2 = Box::new(2);
    assert_eq!(*heap_val_1, 1);
    assert_eq!(*heap_val_2, 2);
}

#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }

    assert_eq!(vec.len(), n);
    assert!(vec.capacity() >= n);

    for i in 0..n {
        assert_eq!(vec[i], i);
    }
}

#[test_case]
fn many_boxes() {
    for i in 0..allocator::HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn many_boxes_long_lived() {
    let long_lived = Box::new(1);
    for i in 0..allocator::HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
        assert_eq!(*long_lived, 1);
    }
}
