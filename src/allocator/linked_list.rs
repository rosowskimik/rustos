use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};

use super::{align_up, Locked};

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        Self { size, next: None }
    }

    #[inline]
    fn start_addr(&self) -> usize {
        self as *const _ as usize
    }

    #[inline]
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }

    /// Checks whether the region represented by `self` can hold an allocation of
    /// `size` bytes with `align`-byte alignment, returning [`Some`] containing
    /// properly aligned start address if so, and [`None`] otherwise.
    fn can_hold(&self, size: usize, align: usize) -> Option<usize> {
        let alloc_start = align_up(self.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size)?;

        // Check that the end address is within bounds
        if alloc_end > self.end_addr() {
            return None;
        }

        // Check whether the remaining space is big enough to fit a new node
        let remaining = self.end_addr() - alloc_end;
        if remaining > 0 && remaining < mem::size_of::<Self>() {
            return None;
        }

        Some(alloc_start)
    }

    /// Adjusts the given layout so that the resulting allocation is also capable
    /// of storing a [`ListNode`].
    ///
    /// Returns the adjusted size and alignment in a tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<Self>())
            .expect("alignment adjustment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<Self>());
        (size, layout.align())
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// Creates an empty `LinkedListAllocator`.
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// # Safety
    /// This method is unsafe because the caller must guarantee that the given
    /// heap bounds are valid. Also, this method must be only called once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// Adds given memory region to the allocator.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are true:
    ///
    /// * `addr` is not proprely aligned.
    ///
    /// * `size` is < size of [`ListNode`].
    ///
    /// # Safety
    ///
    /// This method is unsafe because the caller must guarantee that the given
    /// bounds are valid and that the region is unused.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // make sure that the free region can hold a ListNode
        assert_eq!(
            align_up(addr, mem::align_of::<ListNode>()),
            addr,
            "address is not preprely aligned"
        );
        assert!(
            size >= mem::size_of::<ListNode>(),
            "region is not large enough to hold a ListNode"
        );

        let mut node = ListNode::new(size);
        node.next = self.head.next.take();

        let node_ptr = addr as *mut ListNode;
        unsafe { node_ptr.write(node) };
        self.head.next = Some(unsafe { &mut *node_ptr });
    }

    /// Searches for a free region of at least `size` bytes and removes it from
    /// the list of free regions. Returns [`Some`] containing [`ListNode`]
    /// describing the free region and it's properly aligned start address, or
    /// [`None`] if no free region large enough was found.
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;

        // Iterate over free regions
        while let Some(ref mut node) = current.next {
            // checking whether the current node can hold the requested allocation
            if let Some(alloc_start) = node.can_hold(size, align) {
                // and if so, remove the node from the list and return it
                let next = node.next.take();
                let removed = current.next.take().map(|n| (n, alloc_start));
                current.next = next;

                return removed;
            };

            // otherwise, try the next node
            current = current.next.as_mut().unwrap();
        }

        None
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = ListNode::size_align(layout);
        let mut allocator = self.lock();

        if let Some((node, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start + size;
            let remaining = node.end_addr() - alloc_end;
            if remaining > 0 {
                unsafe { allocator.add_free_region(alloc_end, remaining) };
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = ListNode::size_align(layout);

        self.lock().add_free_region(ptr as usize, size);
    }
}
