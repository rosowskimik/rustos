use core::ptr;

use alloc::alloc::{GlobalAlloc, Layout};

use super::{align_up, Locked};

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialize the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// This method is unsafe because the caller must guarantee that the given
    /// heap bounds are valid. Also, this method must be only called once to
    /// avoid aliasing `&mut` references (which is undefined behavior).
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut alloc = self.lock();

        let alloc_start = align_up(alloc.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > alloc.heap_end {
            ptr::null_mut()
        } else {
            alloc.next = alloc_end;
            alloc.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut alloc = self.lock();

        alloc.allocations -= 1;

        match (ptr as usize).checked_add(layout.size()) {
            // Small optimization to allow immediate reuse of memory
            // if deallocating the latest allocation.
            Some(end) if end == alloc.next => {
                alloc.next = ptr as usize;
            }
            _ => {
                if alloc.allocations == 0 {
                    alloc.next = alloc.heap_start;
                }
            }
        }
    }
}
