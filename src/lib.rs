#![cfg_attr(not(test), no_std)]

use core::num;
use core::ops::{Index, IndexMut};

use gc_headers::{GarbageCollectingHeap, HeapError, Pointer, Tracer};
use gc_headers::HeapError::{IllegalBlock,UnallocatedBlock,OffsetTooBig,MisalignedPointer,ZeroSizeRequest,OutOfMemory};

fn independent_elements_from<T>(i: usize, j: usize, slice: &mut [T]) -> Option<(&mut T, &mut T)> {
    if i == j || i >= slice.len() || j >= slice.len() {
        None
    } else if i < j {
        let (left, right) = slice.split_at_mut(j);
        Some((&mut left[i], &mut right[0]))
    } else {
        let (left, right) = slice.split_at_mut(i);
        Some((&mut right[0], &mut left[j]))
    }
}

#[derive(Copy, Clone, Debug)]
struct BlockInfo {
    start: usize,
    size: usize,
    num_times_copied: usize,
}

#[derive(Copy, Clone, Debug)]
struct BlockTable<const MAX_BLOCKS: usize> {
    block_info: [Option<BlockInfo>; MAX_BLOCKS],
}

impl<const MAX_BLOCKS: usize> Index<usize> for BlockTable<MAX_BLOCKS> {
    type Output = Option<BlockInfo>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.block_info[index]
    }
}

impl<const MAX_BLOCKS: usize> IndexMut<usize> for BlockTable<MAX_BLOCKS> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.block_info[index]
    }
}

impl<const MAX_BLOCKS: usize> BlockTable<MAX_BLOCKS> {
    fn new() -> Self {
        Self {
            block_info: [None; MAX_BLOCKS],
        }
    }

    fn available_block(&self) -> Option<usize> {
        //todo!("Return the lowest numbered unused block");
        for (index, block) in self.block_info.iter().enumerate() {
            if block.is_none() {
                return Some(index); 
            }
        }
        None

    }

    fn blocks_in_use(&self) -> impl Iterator<Item = usize> + '_ {
        (0..MAX_BLOCKS).filter(|b| self.block_info[*b].is_some())
    }

    fn blocks_num_copies(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.blocks_in_use()
            .map(|b| (b, self.block_info[b].unwrap().num_times_copied))
    }
    
    fn address(&self, p: Pointer) -> anyhow::Result<usize, HeapError> {
         //todo!("Find the address, i.e., start + offset, for the Pointer p");
        // Outline
        //
        // 1. If p has a block number that would be an illegal array access, report IllegalBlock.
        if p.block_num() >= MAX_BLOCKS {
            return Err(IllegalBlock(p.block_num(), MAX_BLOCKS - 1));
        }
    
        // 2. If p's block has a None entry, report UnallocatedBlock.
        let pblock = &self.block_info[p.block_num()];
        if pblock.is_none() {
            return Err(UnallocatedBlock(p.block_num()));
        }
    
        // 3. If p's block has an offset that exceeds the size of our block, report OffsetTooBig.
        if p.offset() >= self.block_info[p.block_num()].unwrap().size {
            return Err(OffsetTooBig(p.offset(), p.block_num(), self.block_info[p.block_num()].unwrap().size));
        }
    
        // 4. If p's block size is different than our block in the table, report MisalignedPointer.
        if p.len() != self.block_info[p.block_num()].unwrap().size {
            return Err(MisalignedPointer(p.len(), self.block_info[p.block_num()].unwrap().size, p.block_num()));
        } 
    
        // 5. If none of those errors arises, return the start plus the offset.
        let address = self.block_info[p.block_num()].unwrap().start + p.offset();
    
        Ok(address)
    }
    
    fn allocated_block_ptr(&self, block: usize) -> Option<Pointer> {
        match self.block_info.get(block) {
            None => None,
            Some(info) => info.map(|info| Pointer::new(block, info.size)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct RamHeap<const HEAP_SIZE: usize> {
    heap: [u64; HEAP_SIZE],
    next_address: usize,
}

impl<const HEAP_SIZE: usize> RamHeap<HEAP_SIZE> {
    fn new() -> Self {
        Self {
            heap: [0; HEAP_SIZE],
            next_address: 0,
        }
    }

    fn clear(&mut self) {
        self.next_address = 0;
    }

    fn load(&self, address: usize) -> anyhow::Result<u64, HeapError> {
        //todo!("Return contents of heap at the given address. If address is illegal report it.");
        if address >= HEAP_SIZE {
            if self.next_address == 0{
                return Err(HeapError::IllegalAddress(address, 0));
            }
            else{
                return Err(HeapError::IllegalAddress(address, HEAP_SIZE));
            }
            
        }

        
        return Ok(self.heap[address]);
        
        
    }

    fn store(&mut self, address: usize, value: u64) -> anyhow::Result<(), HeapError> {
        //todo!("Store value in heap at the given address. If address is illegal report it.");
        if address >= HEAP_SIZE {
            return Err(HeapError::IllegalAddress(address, HEAP_SIZE));
        }
        self.heap[address] = value;
        Ok(())
    }


    fn malloc(&mut self, num_words: usize) -> anyhow::Result<usize, HeapError> {
        //todo!("Perform basic malloc");
        // Outline
        //
        // If the request is of size zero, report ZeroSizeRequest
        if num_words == 0{
            return Err(ZeroSizeRequest);
        }
        let new_addy = self.next_address + num_words;
        if new_addy > HEAP_SIZE{
            return Err(OutOfMemory);
        } 
        self.next_address = new_addy;
        return Ok(self.next_address - num_words);
        // Otherwise, calculate the address that will be given for the request to follow.
        // If that exceeds the heap size, report OutOfMemory
        // Otherwise, update `self.next_address` and return the address of the newly allocated memory.
    }

    fn copy(&self, src: &BlockInfo, dest: &mut Self) -> anyhow::Result<BlockInfo, HeapError> {
        //todo!("Copy memory contents from src to dest");
        // Outline
        //
        // Perform a malloc() in dest of the block's size.
        let new_addy = dest.malloc(src.size)?;
        // Store every value from src's block in dest's block.
        for i in 0..src.size {
            let src_address = src.start + i; 
            let value = self.load(src_address)?; 
    
            dest.store(new_addy + i, value)?;
        }
        // Return updated block information, including the starting address and an updated number of copies.
        let block_info = BlockInfo {
            start: new_addy,
            size: src.size,
            num_times_copied: src.num_times_copied + 1, 
        };

        return Ok(block_info);
    }
}

pub struct OnceAndDoneHeap<const HEAP_SIZE: usize, const MAX_BLOCKS: usize> {
    heap: RamHeap<HEAP_SIZE>,
    block_info: BlockTable<MAX_BLOCKS>,
}

impl<const HEAP_SIZE: usize, const MAX_BLOCKS: usize> GarbageCollectingHeap
    for OnceAndDoneHeap<HEAP_SIZE, MAX_BLOCKS>
{
    fn new() -> Self {
        Self {
            heap: RamHeap::new(),
            block_info: BlockTable::new(),
        }
    }

    fn address(&self, p: Pointer) -> anyhow::Result<usize, HeapError> {
        self.block_info.address(p)
    }

    fn load(&self, p: Pointer) -> anyhow::Result<u64, HeapError> {
        self.block_info
            .address(p)
            .and_then(|address| self.heap.load(address))
    }

    fn store(&mut self, p: Pointer, value: u64) -> anyhow::Result<(), HeapError> {
        self.block_info
            .address(p)
            .and_then(|address| self.heap.store(address, value))
    }

    fn blocks_in_use(&self) -> impl Iterator<Item = usize> {
        self.block_info.blocks_in_use()
    }

    fn allocated_block_ptr(&self, block: usize) -> Option<Pointer> {
        self.block_info.allocated_block_ptr(block)
    }

    fn blocks_num_copies(&self) -> impl Iterator<Item = (usize, usize)> {
        self.block_info.blocks_num_copies()
    }

    fn malloc<T: Tracer>(&mut self, num_words: usize, _: &T) -> anyhow::Result<Pointer, HeapError> {
        match self.block_info.available_block() {
            Some(block_num) => {
                let start = self.heap.malloc(num_words)?;
                self.block_info[block_num] = Some(BlockInfo {
                    start,
                    size: num_words,
                    num_times_copied: 0,
                });
                Ok(Pointer::new(block_num, num_words))
            }
            None => Err(HeapError::OutOfBlocks),
        }
    }

    fn assert_no_strays(&self) {}
}

pub struct CopyingHeap<const HEAP_SIZE: usize, const MAX_BLOCKS: usize> {
    heaps: [RamHeap<HEAP_SIZE>; 2],
    block_info: BlockTable<MAX_BLOCKS>,
    active_heap: usize,
}

impl<const HEAP_SIZE: usize, const MAX_BLOCKS: usize> CopyingHeap<HEAP_SIZE, MAX_BLOCKS> {

    fn collect<T: Tracer>(&mut self, tracer: &T) -> anyhow::Result<(), HeapError> {
        let inactive = (self.active_heap + 1) % 2;
        let (src, dest) =
            independent_elements_from(self.active_heap, inactive, &mut self.heaps).unwrap();
        //todo!("Implement copying collection.");
        // Outline
        //
        // 1. Run the `trace()` method of the `tracer` to find blocks in use.
        let mut blocks = [false; MAX_BLOCKS];
        tracer.trace(&mut blocks);
 
        // 2. For each block in use:
        //    * Copy the block from `src` to `dest`.
        for (i, block) in blocks.iter().enumerate(){
            if *block{
                let new_block_info = src.copy(&self.block_info[i].unwrap(), dest)?;
                self.block_info[i] = Some(new_block_info);
            }
            else{
                self.block_info[i] = None;
            }
        };

        // Update block_info[i]
            // * If the block is not in use, reset it to None.
            // * If the block gets copied, overwrite its entry
            //   with what you get from copy().

        // 3. Clear the active heap.
        self.heaps[self.active_heap].clear();
 
        // 4. Set `self.active_heap` to point at the newly active heap.
        self.active_heap = inactive;
       
        Ok(())
    }

    
}

impl<const HEAP_SIZE: usize, const MAX_BLOCKS: usize> GarbageCollectingHeap
    for CopyingHeap<HEAP_SIZE, MAX_BLOCKS>
{
    fn new() -> Self {
        Self {
            heaps: [RamHeap::new(); 2],
            block_info: BlockTable::new(),
            active_heap: 0,
        }
    }

    fn address(&self, p: Pointer) -> anyhow::Result<usize, HeapError> {
        self.block_info.address(p)
    }

    fn load(&self, p: Pointer) -> anyhow::Result<u64, HeapError> {
        self.block_info
            .address(p)
            .and_then(|address| self.heaps[self.active_heap].load(address))
    }

    fn store(&mut self, p: Pointer, value: u64) -> anyhow::Result<(), HeapError> {
        self.block_info
            .address(p)
            .and_then(|address| self.heaps[self.active_heap].store(address, value))
    }

    fn blocks_in_use(&self) -> impl Iterator<Item = usize> {
        self.block_info.blocks_in_use()
    }

    fn allocated_block_ptr(&self, block: usize) -> Option<Pointer> {
        self.block_info.allocated_block_ptr(block)
    }

    fn blocks_num_copies(&self) -> impl Iterator<Item = (usize, usize)> {
        self.block_info.blocks_num_copies()
    }
    fn malloc<T: Tracer>(
        &mut self,
        num_words: usize,
        tracer: &T,
    ) -> anyhow::Result<Pointer, HeapError> {
        //todo!("Implement malloc");
        // Outline
        //
        // 1. Find an available block number
        //    * If none are available, perform a collection.
        //    * If none are still available, report out of blocks.
        if num_words == 0{
            return Err(ZeroSizeRequest)
        }

        
        let block_num = match self.block_info.available_block() {
            Some(block_num) => block_num,
            None => match self.collect(tracer) {
                Ok(_) => match self.block_info.available_block() {
                    Some(block_num) => block_num,
                    None => return Err(HeapError::OutOfBlocks),
                },
                Err(e) => return Err(e),
            },
        };
    
         // 2. Perform a malloc in the currently active heap.
        //    * If no space is available, perform a collection by calling self.collect().
        //    * If no space is still available, report out of memory.
       
        let address = match self.heaps[self.active_heap].malloc(num_words) {
            Ok(addy) => addy,
            Err(_) => match self.collect(tracer) {
                Ok(_) => match self.heaps[self.active_heap].malloc(num_words) {
                    Ok(addy2) => addy2,
                    Err(_) => return Err(HeapError::OutOfMemory),
                },
                Err(_) => return Err(HeapError::OutOfMemory),
            },
        };
    
        // 3. Create entry in the block table for the newly allocated block.
        self.block_info[block_num] = Some(BlockInfo {
            start: address,
            size: num_words,
            num_times_copied: 0,
        });
        // 4. Return a pointer to the newly allocated block.
        Ok(Pointer::new(block_num, num_words))
    }
     

    fn assert_no_strays(&self) {
        assert!(self.heaps[(self.active_heap + 1) % 2].next_address == 0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GenerationalHeap<
    const HEAP_SIZE: usize,
    const MAX_BLOCKS: usize,
    const MAX_COPIES: usize,
> {
    gen_0: [RamHeap<HEAP_SIZE>; 2],
    gen_1: [RamHeap<HEAP_SIZE>; 2],
    block_info: BlockTable<MAX_BLOCKS>,
    active_gen_0: usize,
    active_gen_1: usize,
}

impl<const HEAP_SIZE: usize, const MAX_BLOCKS: usize, const MAX_COPIES: usize>
    GenerationalHeap<HEAP_SIZE, MAX_BLOCKS, MAX_COPIES>
{
    fn active_inactive_gen_0_gen_1(
        &mut self,
    ) -> (
        &mut RamHeap<HEAP_SIZE>,
        &mut RamHeap<HEAP_SIZE>,
        &mut RamHeap<HEAP_SIZE>,
        &mut RamHeap<HEAP_SIZE>,
        &mut BlockTable<MAX_BLOCKS>,
    ) {
        let inactive_0 = (self.active_gen_0 + 1) % 2;
        let inactive_1 = (self.active_gen_1 + 1) % 2;
        let (active_0, inactive_0) =
            independent_elements_from(self.active_gen_0, inactive_0, &mut self.gen_0).unwrap();
        let (active_1, inactive_1) =
            independent_elements_from(self.active_gen_1, inactive_1, &mut self.gen_1).unwrap();
        (
            active_0,
            inactive_0,
            active_1,
            inactive_1,
            &mut self.block_info,
        )
    }

    fn heap_and_gen_for(&self, block_num: usize) -> anyhow::Result<(usize, usize), HeapError> {
        if block_num >= MAX_BLOCKS {
            Err(HeapError::IllegalBlock(block_num, MAX_BLOCKS - 1))
        } else {
            match self.block_info[block_num] {
                Some(block_info) => Ok(if block_info.num_times_copied > MAX_COPIES {
                    (self.active_gen_1, 1)
                } else {
                    (self.active_gen_0, 0)
                }),
                None => Err(HeapError::UnallocatedBlock(block_num)),
            }
        }
    }

    fn collect_gen_0<T: Tracer>(&mut self, tracer: &T) -> anyhow::Result<(), HeapError> {
        // This line is necessary because the borrow checker disallows mutable references to 
        // multiple array elements. By modifying the variables below, you should be able to
        // achieve everything necessary.
        let (active_0, inactive_0, active_1, inactive_1, block_info) =
            self.active_inactive_gen_0_gen_1();
        todo!("Complete implementation.");
        // Outline
        //
        // 1. Call the tracer to find out what blocks are in use.
        // 2. For each block in use:
        //    * If it has been copied MAX_COPIES times
        //      * You'll need a variable to track whether you have already performed a generation 1 collection.
        //      * If so, just return the error - multiple generation 1 collections will not be productive
        //      * If not, copy into the active generation 1 heap.
        //      * If that heap is out of space, perform a generation 1 collection by calling self.collect_gen_1().
        //      * After the generation 1 collection, try copying it into the inactive generation 1 heap.
        //    * If not, copy it into the inactive generation 0 heap.
        // 3. Clear the active generation 0 heap.
        // 4. Update self.active_gen_0 to the other heap.
        // 5. If there was a generation 1 collection, update self.active_gen_1 to the other heap.
    }

    fn collect_gen_1(
        blocks_used: &[bool; MAX_BLOCKS],
        block_info: &mut BlockTable<MAX_BLOCKS>,
        src: &RamHeap<HEAP_SIZE>,
        dest: &mut RamHeap<HEAP_SIZE>,
    ) -> anyhow::Result<(), HeapError> {
        todo!("Complete implementation.");
        // Outline
        //
        // 1. For each block in use:
        //    * If it has been copied more than MAX_COPIES times, copy it to `dest`
        // 2. Clear the `src` heap.
    }
}

impl<const HEAP_SIZE: usize, const MAX_BLOCKS: usize, const MAX_COPIES: usize> GarbageCollectingHeap
    for GenerationalHeap<HEAP_SIZE, MAX_BLOCKS, MAX_COPIES>
{
    fn new() -> Self {
        Self {
            gen_0: [RamHeap::new(); 2],
            gen_1: [RamHeap::new(); 2],
            block_info: BlockTable::new(),
            active_gen_0: 0,
            active_gen_1: 0,
        }
    }

    fn load(&self, p: Pointer) -> anyhow::Result<u64, HeapError> {
        let (heap, gen) = self.heap_and_gen_for(p.block_num())?;
        let address = self.block_info.address(p)?;
        (if gen == 0 {
            &self.gen_0[heap]
        } else {
            &self.gen_1[heap]
        })
        .load(address)
    }

    fn store(&mut self, p: Pointer, value: u64) -> anyhow::Result<(), HeapError> {
        let (heap, gen) = self.heap_and_gen_for(p.block_num())?;
        let address = self.block_info.address(p)?;
        (if gen == 0 {
            &mut self.gen_0[heap]
        } else {
            &mut self.gen_1[heap]
        })
        .store(address, value)
    }

    fn address(&self, p: Pointer) -> anyhow::Result<usize, HeapError> {
        self.block_info.address(p)
    }

    fn blocks_in_use(&self) -> impl Iterator<Item = usize> {
        self.block_info.blocks_in_use()
    }

    fn allocated_block_ptr(&self, block: usize) -> Option<Pointer> {
        self.block_info.allocated_block_ptr(block)
    }

    fn blocks_num_copies(&self) -> impl Iterator<Item = (usize, usize)> {
        self.block_info.blocks_num_copies()
    }

    fn malloc<T: Tracer>(
        &mut self,
        num_words: usize,
        tracer: &T,
    ) -> anyhow::Result<Pointer, HeapError> {
        todo!("Implement generational malloc");
        // Outline
        //
        // 1. Find an available block number
        //    * If none are available, perform a collection by calling self.collect_gen_0().
        //    * If none are still available, report out of blocks.
        // 2. Perform a generation zero malloc.
        //    * If no space is available, perform a collection by calling self.collect_gen_0().
        //    * If no space is still available, report out of memory.
        // 3. Create entry in the block table for the newly allocated block.
        // 4. Return a pointer to the newly allocated block.
    }

    fn assert_no_strays(&self) {
        assert!(self.gen_0[(self.active_gen_0 + 1) % 2].next_address == 0);
        assert!(self.gen_1[(self.active_gen_1 + 1) % 2].next_address == 0);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use core::fmt::Debug;

    use super::*;
    use test_tracer::TestTracer;

    const HEAP_SIZE: usize = 96;
    const MAX_BLOCKS: usize = 12;

    // Level 1 Unit Tests

    #[test]
    fn block_table_test() {
        let mut table = BlockTable::<5>::new();
        assert_eq!(table.available_block().unwrap(), 0);
        table[0] = Some(BlockInfo { start: 3, size: 2, num_times_copied: 0 });
        assert_eq!(table.available_block().unwrap(), 1);
        table[2] = Some(BlockInfo { start: 5, size: 3, num_times_copied: 0 });
        assert_eq!(table.available_block().unwrap(), 1);
        table[1] = Some(BlockInfo { start: 8, size: 2, num_times_copied: 0 });
        assert_eq!(table.available_block().unwrap(), 3);

        let p = Pointer::new(0, 2);
        for (i, ptr) in p.iter().enumerate() {
            assert_eq!(table.address(ptr).unwrap(), i + 3);
        }
        let end_ptr = p.iter().last().unwrap();
        table[0] = Some(BlockInfo {start: 3, size: 1, num_times_copied: 0});
        assert_eq!(table.address(p), Err(HeapError::MisalignedPointer(2, 1, 0)));
        assert_eq!(table.address(end_ptr), Err(HeapError::OffsetTooBig(1, 0, 1)));

        let p = Pointer::new(5, 2);
        assert_eq!(table.address(p), Err(HeapError::IllegalBlock(5, 4)));

        let p = Pointer::new(3, 2);
        assert_eq!(table.address(p), Err(HeapError::UnallocatedBlock(3)));
    }

    #[test]
    fn basic_allocation_test() {
        let mut blocks2ptrs = HashMap::new();
        let mut allocator = OnceAndDoneHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        test_initial_allocation(&mut allocator, &mut tracer, &mut blocks2ptrs);
    }

    #[test]
    fn out_of_blocks_test() {
        let mut blocks2ptrs = HashMap::new();
        let mut allocator = OnceAndDoneHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        test_initial_allocation(&mut allocator, &mut tracer, &mut blocks2ptrs);
        test_out_of_blocks(&mut allocator, &mut tracer);
    }

    #[test]
    fn test_bad_address_error() {
        let mut allocator = RamHeap::<HEAP_SIZE>::new();
        match allocator.load(HEAP_SIZE + 1) {
            Ok(_) => panic!("This should have been an IllegalAddress error."),
            Err(e) => assert_eq!(e, HeapError::IllegalAddress(HEAP_SIZE + 1, 0))
        }

        allocator.malloc(96).unwrap();
        match allocator.load(HEAP_SIZE + 1) {
            Ok(_) => panic!("This should have been an IllegalAddress error."),
            Err(e) => assert_eq!(e, HeapError::IllegalAddress(HEAP_SIZE + 1, HEAP_SIZE))
        }
    }

    // Level 2 Unit Tests

    #[test]
    fn deallocation_test() {
        let mut blocks2ptrs = HashMap::new();
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        test_initial_allocation(&mut allocator, &mut tracer, &mut blocks2ptrs);
        allocator.assert_no_strays();
        test_out_of_blocks(&mut allocator, &mut tracer);
        test_remove_half(&mut allocator, &mut tracer, &mut blocks2ptrs);
    }

    #[test]
    fn collection_test() {
        let mut blocks2ptrs = HashMap::new();
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        test_initial_allocation(&mut allocator, &mut tracer, &mut blocks2ptrs);
        test_out_of_blocks(&mut allocator, &mut tracer);
        test_remove_half(&mut allocator, &mut tracer, &mut blocks2ptrs);
        test_force_collection(&mut allocator, &mut tracer, &mut blocks2ptrs);
        allocator.assert_no_strays();
    }

    #[test]
    fn full_test() {
        let mut blocks2ptrs = HashMap::new();
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        test_initial_allocation(&mut allocator, &mut tracer, &mut blocks2ptrs);
        test_remove_half(&mut allocator, &mut tracer, &mut blocks2ptrs);
        test_force_collection(&mut allocator, &mut tracer, &mut blocks2ptrs);
        test_fill_ram(&mut allocator, &mut tracer, &mut blocks2ptrs);
        allocator.assert_no_strays();
        test_out_of_ram(&mut allocator, &mut tracer);
    }

    #[test]
    fn test_no_blocks_error() {
        let mut blocks2ptrs = HashMap::new();
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        test_initial_allocation(&mut allocator, &mut tracer, &mut blocks2ptrs);
        test_out_of_blocks(&mut allocator, &mut tracer);
    }

    #[test]
    fn test_zero_size_error() {
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let tracer = TestTracer::default();
        match allocator.malloc(0, &tracer) {
            Ok(_) => panic!("This should have been a zero-size error"),
            Err(e) => assert_eq!(e, HeapError::ZeroSizeRequest),
        }
    }

    #[test]
    fn test_illegal_block_error() {
        let allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let bad_ptr = Pointer::new(MAX_BLOCKS, 1);
        match allocator.load(bad_ptr) {
            Ok(_) => panic!("This should have been an error"),
            Err(e) => assert_eq!(e, HeapError::IllegalBlock(MAX_BLOCKS, MAX_BLOCKS - 1))
        }
    }

    #[test]
    fn test_unallocated_block_error() {
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let tracer = TestTracer::default();
        let p = allocator.malloc(1, &tracer).unwrap();
        let bad_ptr = Pointer::new(p.block_num() + 1, 1);
        match allocator.load(bad_ptr) {
            Ok(_) => panic!("This should have been an UnallocatedBlock error"),
            Err(e) => assert_eq!(e, HeapError::UnallocatedBlock(bad_ptr.block_num()))
        }
    }

    #[test]
    fn test_offset_error() {
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        let p = tracer.allocate_next(HEAP_SIZE, &mut allocator).unwrap();
        let s = p.iter().skip(1).next().unwrap();
        tracer.deallocate_next().unwrap();
        tracer.allocate_next(1, &mut allocator).unwrap();
        let q = tracer.allocate_next(1, &mut allocator).unwrap();
        assert_eq!(p.block_num(), q.block_num());
        match allocator.load(s) {
            Ok(_) => panic!("This should have been an OffsetTooBig error"),
            Err(e) => assert_eq!(e, HeapError::OffsetTooBig(1, p.block_num(), 1))
        }
    }

    #[test]
    fn test_misaligned_pointer_error() {
        let mut allocator = CopyingHeap::<HEAP_SIZE, MAX_BLOCKS>::new();
        let mut tracer = TestTracer::default();
        let p = tracer.allocate_next(HEAP_SIZE, &mut allocator).unwrap();
        tracer.deallocate_next().unwrap();
        tracer.allocate_next(1, &mut allocator).unwrap();
        let q = tracer.allocate_next(1, &mut allocator).unwrap();
        assert_eq!(p.block_num(), q.block_num());
        match allocator.load(p) {
            Ok(_) => panic!("This should have been a MisalignedPointer error"),
            Err(e) => assert_eq!(e, HeapError::MisalignedPointer(HEAP_SIZE, 1, p.block_num()))
        }
    }

    fn test_initial_allocation<H: GarbageCollectingHeap>(
        allocator: &mut H,
        tracer: &mut TestTracer,
        blocks2ptrs: &mut HashMap<usize, Pointer>,
    ) {
        for (block_num, request) in [2, 10, 4, 8, 6, 12, 6, 24, 4, 8, 2, 8].iter().enumerate() {
            println!("block: {block_num} request: {request}");
            let allocated_ptr = tracer.allocate_next(*request, allocator).unwrap();
            assert_eq!(block_num, allocated_ptr.block_num());
            assert_eq!(*request, allocated_ptr.len());
            blocks2ptrs.insert(block_num, allocated_ptr);
            assert_eq!(blocks2ptrs.len(), allocator.num_allocated_blocks());
            ensure_non_overlapping(blocks2ptrs, allocator);
        }
        ensure_all_match(blocks2ptrs, allocator);
        assert_eq!(total_words_allocated(blocks2ptrs), 94);
        test_load_store(&blocks2ptrs, allocator);
        assert_eq!(allocator.num_allocated_blocks(), 12);
    }

    fn test_remove_half<H: GarbageCollectingHeap>(
        allocator: &mut H,
        tracer: &mut TestTracer,
        blocks2ptrs: &mut HashMap<usize, Pointer>,
    ) {
        for _ in 0..(tracer.len() / 2) {
            let removed = tracer.deallocate_next_even().unwrap();
            assert!(blocks2ptrs.contains_key(&removed.block_num()));
            blocks2ptrs.remove(&removed.block_num());
        }
        test_load_store(&blocks2ptrs, allocator);
        assert_eq!(allocator.num_allocated_blocks(), 12);
    }

    fn test_force_collection<H: GarbageCollectingHeap>(
        allocator: &mut H,
        tracer: &mut TestTracer,
        blocks2ptrs: &mut HashMap<usize, Pointer>,
    ) {
        let ptr = tracer.allocate_next(4, allocator).unwrap();
        assert!(!blocks2ptrs.contains_key(&ptr.block_num()));
        blocks2ptrs.insert(ptr.block_num(), ptr);
        assert_eq!(allocator.num_allocated_blocks(), 7);
        assert_eq!(tracer.len(), allocator.num_allocated_blocks());
    }

    fn test_fill_ram<H: GarbageCollectingHeap>(
        allocator: &mut H,
        tracer: &mut TestTracer,
        blocks2ptrs: &mut HashMap<usize, Pointer>,
    ) {
        let ptr = tracer.allocate_next(68, allocator).unwrap();
        assert!(!blocks2ptrs.contains_key(&ptr.block_num()));
        blocks2ptrs.insert(ptr.block_num(), ptr);
        assert_eq!(allocator.num_allocated_blocks(), 8);
        assert_eq!(tracer.total_allocated(), 96);
    }

    fn test_out_of_ram<H: GarbageCollectingHeap>(allocator: &mut H, tracer: &mut TestTracer) {
        match tracer.allocate_next(1, allocator) {
            Ok(_) => panic!("Should be an out of memory error!"),
            Err(e) => assert_eq!(e, HeapError::OutOfMemory),
        }
    }

    fn ensure_all_match<H: GarbageCollectingHeap>(
        blocks2ptrs: &HashMap<usize, Pointer>,
        allocator: &H,
    ) {
        for (block, ptr) in blocks2ptrs.iter() {
            assert_eq!(allocator.allocated_block_ptr(*block).unwrap(), *ptr);
        }
    }

    fn ensure_non_overlapping<H: GarbageCollectingHeap>(
        blocks2ptrs: &HashMap<usize, Pointer>,
        allocator: &H,
    ) {
        let mut memory_locations = (0..HEAP_SIZE).collect::<HashSet<_>>();
        for ptr in blocks2ptrs.values() {
            for inner in ptr.iter() {
                let addr = allocator.address(inner).unwrap();
                assert!(memory_locations.contains(&addr));
                memory_locations.remove(&addr);
            }
        }
    }

    fn test_load_store<H: GarbageCollectingHeap>(
        blocks2ptrs: &HashMap<usize, Pointer>,
        allocator: &mut H,
    ) {
        let mut value = 0;
        for p in blocks2ptrs.values() {
            for pt in p.iter() {
                allocator.store(pt, value).unwrap();
                assert_eq!(value, allocator.load(pt).unwrap());
                value += 1;
            }
        }

        value = 0;
        for p in blocks2ptrs.values() {
            for pt in p.iter() {
                assert_eq!(value, allocator.load(pt).unwrap());
                value += 1;
            }
        }
    }

    fn total_words_allocated(blocks2ptrs: &HashMap<usize, Pointer>) -> usize {
        blocks2ptrs.values().map(|p| p.len()).sum()
    }

    fn test_out_of_blocks<H: GarbageCollectingHeap>(allocator: &mut H, tracer: &mut TestTracer) {
        match tracer.allocate_next(1, allocator) {
            Ok(_) => panic!("Allocator should be out of space - this should be an error"),
            Err(e) => assert_eq!(e, HeapError::OutOfBlocks),
        }
    }

    // Level 3 Unit Test

    #[test]
    fn generational_test() {
        let mut allocator = GenerationalHeap::<100, 120, 2>::new();
        let mut tracer = TestTracer::default();
        let mut blocks2ptrs = HashMap::new();
        allocate_many(40, &mut allocator, &mut tracer, &mut blocks2ptrs);
        allocator.assert_no_strays();

        assert_eq!(blocks2ptrs.len(), allocator.num_allocated_blocks());
        for (_, c) in allocator.blocks_num_copies() {
            assert_eq!(c, 0);
        }
        
        for expected_copies in 1..=3 {
            force_copy_n(expected_copies, &mut allocator, &mut tracer, &mut blocks2ptrs);
            for (b, c) in allocator.blocks_num_copies() {
                if b >= expected_copies && b < blocks2ptrs.len() {
                    assert_eq!(c, expected_copies);
                }
                if let Some(p) = blocks2ptrs.get(&b) {
                    assert_eq!(p.len() as u64, allocator.load(*p).unwrap());
                }
            }
            allocator.assert_no_strays();
        }

        allocate_many(38, &mut allocator, &mut tracer, &mut blocks2ptrs);
        allocator.assert_no_strays();
        
        for _ in 1..=4 {
            tracer.deallocate_next().unwrap();
            tracer.allocate_next(1, &mut allocator).unwrap();
            allocator.assert_no_strays();
        }

        for (_, c) in allocator.blocks_num_copies() {
            assert!(c <= 3);
        }   

        tracer.deallocate_any_that(|p| p.len() != 3);

        tracer.allocate_next(1, &mut allocator).unwrap();
        allocator.assert_no_strays();
        for (_, c) in allocator.blocks_num_copies() {
            assert!(c <= 4);
        } 
    }

    fn allocate_many<H: GarbageCollectingHeap + Debug>(
        num_allocations: usize,
        allocator: &mut H,
        tracer: &mut TestTracer,
        blocks2ptrs: &mut HashMap<usize, Pointer>,
    ) {
        for i in 0..num_allocations {
            let size = i % 4 + 1;
            let p = tracer.allocate_next(size, allocator).unwrap();
            blocks2ptrs.insert(p.block_num(), p);
            for addr in p.iter() {
                allocator.store(addr, size as u64).unwrap();
            }
        }
    }

    fn force_copy_n<H: GarbageCollectingHeap + Debug>(
        n: usize,
        allocator: &mut H,
        tracer: &mut TestTracer,
        blocks2ptrs: &mut HashMap<usize, Pointer>,
    ) {
        let d = tracer.deallocate_next().unwrap();
        assert_eq!(n, d.len());
        blocks2ptrs.remove(&d.block_num());
        let p = tracer.allocate_next(n, allocator).unwrap();
        blocks2ptrs.insert(p.block_num(), p);
        allocator.store(p, n as u64).unwrap();
    }
}
