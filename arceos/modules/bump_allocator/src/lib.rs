#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    base: usize,
    size: usize,
    b_pos: usize,
    p_pos: usize,
    b_count: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            base: 0,
            size: 0,
            b_pos: 0,
            p_pos: 0,
            b_count: 0,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.base = start;
        self.size = size;
        self.b_pos = start;
        self.p_pos = start + size;
        self.b_count = 0;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> allocator::AllocResult {
        Ok(())
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: core::alloc::Layout) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let size = layout.size();
        let pos = self.b_pos + size;
        if pos > self.p_pos {
            return Err(allocator::AllocError::NoMemory);
        }
        let addr = self.b_pos;
        self.b_pos = pos;
        self.b_count += 1;
        unsafe { Ok(core::ptr::NonNull::new_unchecked(addr as *mut u8)) }
    }

    fn dealloc(&mut self, pos: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        let pos = pos.as_ptr() as usize;
        if pos > self.b_pos {
            return;
        }
        self.b_count -= 1;
        if self.b_count == 0 {
            self.b_pos = self.base;
        }
    }

    fn total_bytes(&self) -> usize {
        0
    }

    fn used_bytes(&self) -> usize {
        0
    }

    fn available_bytes(&self) -> usize {
        0
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> allocator::AllocResult<usize> {
        let pos = self.p_pos - num_pages * Self::PAGE_SIZE;
        if pos < self.b_pos {
            return Err(allocator::AllocError::NoMemory);
        }
        self.p_pos = pos;
        return Ok(pos);
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        ()
    }

    fn total_pages(&self) -> usize {
        0
    }

    fn used_pages(&self) -> usize {
        0
    }

    fn available_pages(&self) -> usize {
        0
    }
}
