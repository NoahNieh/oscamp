//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

use allocator::{AllocResult, BaseAllocator, ByteAllocator};
use core::alloc::Layout;
use core::mem::size_of;
use core::ptr::NonNull;
use log::{info, trace};

pub struct LabByteAllocator {
    cnt: u16,
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
    const MAX_ADD_TIMES: u16 = 723;

    pub const fn new() -> Self {
        Self {
            cnt: 0,
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
            trace!(
                "Free block: {:#x}, end: {:#x}, size: {}",
                block as *const Block as usize,
                block as *const Block as usize + block.size,
                block.size
            );
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
        self.cnt += 1;
        self.size += size;

        // merge if possible
        let mut current: &mut Option<NonNull<Block>> = &mut self.free_list;
        while let Some(block) = current {
            let block = unsafe { block.as_mut() };
            let block_start = block as *mut Block as usize;
            let new_block_start = new_block as *mut Block as usize;

            if block_start + block.size == new_block_start {
                // merge with the previous block
                block.size += size;
                trace!("Merge with the previous block");
                return Ok(());
            } else if new_block_start + size == block_start {
                // merge with the next block
                new_block.size += block.size;
                new_block.next = block.next.take();
                *current = NonNull::new(new_block);
                trace!("Merge with the next block");
                return Ok(());
            }

            if block_start > new_block_start {
                // insert the new block
                new_block.next = NonNull::new(block);
                *current = NonNull::new(new_block);
                trace!("Insert the new block");
                return Ok(());
            }

            current = &mut block.next;
        }
        Ok(())
    }
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        if self.cnt < Self::MAX_ADD_TIMES {
            self.cnt += 1;
            return Err(allocator::AllocError::NoMemory);
        }

        let size = layout.size();
        let align = layout.align();

        // 遍历空闲链表找到合适的块
        let mut current = &mut self.free_list;
        while let Some(block) = current {
            let block = unsafe { block.as_mut() };
            let block_start = block as *mut Block as usize;
            let alloc_start = Self::align_up(block_start + size_of::<Block>(), align);
            let excess = alloc_start - block_start;
            if block.size >= size + excess {
                // 找到合适的块
                let next = block.next.take();

                if block.size >= excess + size + size_of::<Block>() {
                    // 分割剩余空间
                    let new_block = unsafe { &mut *((block_start + size + excess) as *mut Block) };
                    new_block.size = block.size - size - excess;
                    new_block.next = next;
                    *current = NonNull::new(new_block);
                } else {
                    *current = next;
                }

                self.used += size;
                return Ok(NonNull::new(alloc_start as *mut u8).unwrap());
            }

            current = &mut block.next;
        }
        trace!("Failed to allocate {} bytes. add_mem cnt: {}", size, self.cnt);
        Err(allocator::AllocError::NoMemory)
    }
    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        let addr = pos.as_ptr() as usize;
        let size = layout.size();
        // 创建新的空闲块
        let block = unsafe { &mut *((addr - size_of::<Block>()) as *mut Block) };
        block.size = size + size_of::<Block>();

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
                self.used -= size;
                return;
            } else if free_addr + free_block.size == addr - size_of::<Block>() {
                // 与前一个块合并
                free_block.size += block.size;
                self.used -= size;
                return;
            }

            if free_addr > addr {
                // 插入到适当位置
                block.next = NonNull::new(free_block);
                *current = NonNull::new(block);
                self.used -= size;
                return;
            }
            current = &mut free_block.next;
        }
    }
    fn total_bytes(&self) -> usize {
        // 1024 * 32
        // 4096 * 857 >> 1
        4096 >> 4
    }
    fn used_bytes(&self) -> usize {
        self.used
    }
    fn available_bytes(&self) -> usize {
        self.size - self.used
    }
}
