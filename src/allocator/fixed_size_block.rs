use alloc::alloc::{GlobalAlloc, Layout};
use core::{
    mem,
    ptr::{self, NonNull},
};

use super::Locked;

/// Possible block sizes avaiable for the allocator.
///
/// The sizes must be power of 2 because they are also used as the block alignment.
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// Finds the minimum block size for the given layout returning the index
/// into the `BLOCK_SIZES` arraya containing it, or `None` if no suitable
/// block size was found.
fn find_block_index(layout: Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

struct ListNode {
    next: Option<&'static mut ListNode>,
}

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// Creates an empty [`FixedSizeBlockAllocator`].
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        Self {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initializes the [`FixedSizeBlockAllocator`] from the given heap bounds.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the caller must guarantee that the given
    /// heap bounds are valid. Also, this method must be only called once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_allocator
            .init(heap_start as *mut u8, heap_size);
    }

    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        match find_block_index(layout) {
            Some(index) => match allocator.list_heads[index].take() {
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    node as *mut ListNode as *mut u8
                }
                None => {
                    // block list is empty, allocate new block with fallback allocator
                    let block_size = BLOCK_SIZES[index];

                    // block's alignment is the same as its size
                    let layout = Layout::from_size_align(block_size, block_size).unwrap();
                    allocator.fallback_alloc(layout)
                }
            },
            // Allocation size is too big, use fallback allocator
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        match find_block_index(layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                // Make sure that this block has correct size and alignment
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                unsafe { new_node_ptr.write(new_node) };
                allocator.list_heads[index] = unsafe { Some(&mut *new_node_ptr) };
            }
            None => unsafe {
                allocator
                    .fallback_allocator
                    .deallocate(NonNull::new_unchecked(ptr), layout);
            },
        }
    }
}
