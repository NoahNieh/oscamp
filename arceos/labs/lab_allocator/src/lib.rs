//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

use allocator::{AllocResult, BaseAllocator, ByteAllocator, TlsfByteAllocator};
use core::alloc::Layout;
use core::mem::size_of;
use core::ptr::NonNull;
use log::{info, trace};

pub struct LabByteAllocator {
    start: usize,
    size: usize,
    used: usize,
    free_list: Option<NonNull<Block>>,
}

struct Block {
    size: usize,
    next: Option<NonNull<Block>>,
}

unsafe impl Send for LabByteAllocator {}

impl LabByteAllocator {
    pub const fn new() -> Self {
        Self {
            start: 0,
            size: 0,
            used: 0,
            free_list: None,
        }
    }

    // 对齐向上取整
    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }

    fn print_free_list(&self) {
        let mut current = self.free_list;
        while let Some(block) = current {
            let block = unsafe { block.as_ref() };
            trace!("Free block: {:#x}, size: {}", block as *const Block as usize, block.size);
            current = block.next;
        }
    }
}

impl BaseAllocator for LabByteAllocator {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.size = size;
        self.used = 0;

        let block = unsafe { &mut *(start as *mut Block) };
        block.size = size;
        block.next = None;
        self.free_list = NonNull::new(block);
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        let new_block = unsafe { &mut *(start as *mut Block) };
        new_block.size = size;

        new_block.next = self.free_list.take();
        self.free_list = NonNull::new(new_block);

        self.size += size;
        Ok(())
    }

}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        trace!("Allocating {:?}", layout);
        let size = layout.size();
        let align = layout.align();

        // 遍历空闲链表找到合适的块
        let mut current = &mut self.free_list;
        while let Some(block) = current {
            let block = unsafe { block.as_mut() };
            let block_start = block as *mut Block as usize;
            let alloc_start = Self::align_up(block_start + size_of::<Block>(), align);
            let excess = alloc_start - block_start;
            trace!(
                "block_start: {:#x}, alloc_start: {:#x}, excess: {}, block_size: {}",
                block_start, alloc_start, excess, block.size
            );
            if block.size >= size + excess {
                trace!("Found block, size: {}", block.size);
                // 找到合适的块
                let next = block.next.take();

                if block.size >= excess + size + size_of::<Block>() {
                    // 分割剩余空间
                    let new_block = unsafe { &mut *((block_start + size + excess) as *mut Block) };
                    new_block.size = block.size - size - excess;
                    new_block.next = next;
                    *current = NonNull::new(new_block);
                    trace!("Split block, new block size: {}", new_block.size);
                } else {
                    *current = next;
                }

                self.used += size;
                trace!("Allocated {} bytes", size);
                return Ok(NonNull::new(alloc_start as *mut u8).unwrap());
            }

            current = &mut block.next;
        }

        Err(allocator::AllocError::NoMemory)
    }
    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        let addr = pos.as_ptr() as usize;
        let size = layout.size();
        // 创建新的空闲块
        let block = unsafe { &mut *((addr - size_of::<Block>()) as *mut Block) };
        block.size = size + size_of::<Block>();
        trace!("Deallocating {:?} at {:#x}, block_size: {}", layout, addr, block.size);

        // 插入到空闲链表并尝试合并
        let mut current = &mut self.free_list;
        while let Some(free_block) = current {
            let free_block = unsafe { free_block.as_mut() };
            let free_addr = free_block as *mut _ as usize;

            if addr + size == free_addr {
                // 与后一个块合并
                block.size += free_block.size;
                block.next = free_block.next.take();
                *current = NonNull::new(block);
                trace!("Merged with previous block, new size: {}", free_block.size);
                self.used -= size;
                return;
            } else if free_addr + free_block.size == addr - size_of::<Block>() {
                // 与前一个块合并
                free_block.size += block.size;
                self.used -= size;
                trace!("Merged with previous block, new size: {}", free_block.size);
                return;
            }

            if free_addr > addr {
                // 插入到适当位置
                block.next = NonNull::new(free_block);
                *current = NonNull::new(block);
                trace!("Inserted block size: {}", block.size);
                self.used -= size;
                return;
            }
            current = &mut free_block.next;
        }
    }
    fn total_bytes(&self) -> usize {
        self.size
    }
    fn used_bytes(&self) -> usize {
        self.used
    }
    fn available_bytes(&self) -> usize {
        self.size - self.used
    }
}
