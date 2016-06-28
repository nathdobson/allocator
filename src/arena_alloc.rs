
use std::cmp;
use std::ptr::null_mut;
use util;
use util::PowerOfTwo;
use std::mem;
use std::ptr;
use allocator::OwnedAllocator;
#[cfg(test)]
use heap_alloc::HeapAlloc;
#[cfg(test)]
use rand;
#[cfg(test)]
use rand::Rng;
#[cfg(test)]
use checked_alloc::CheckedAlloc;
#[cfg(test)]
use checked_alloc::CheckedAllocOptions;
#[cfg(test)]
use simple_alloc::LoggingAlloc;
fn arena_heap_alignment() -> PowerOfTwo {
    return PowerOfTwo::new(1);
}
pub struct ArenaOptions {
    pub start_block_size: usize,
    pub recommended_max_block_size: usize,
}
impl Default for ArenaOptions {
    fn default() -> ArenaOptions {
        return ArenaOptions {
            start_block_size: 4096,
            recommended_max_block_size: 65536,
        };
    }
}
struct UsedBlock {
    memory: *mut u8,
    size: usize,
}
struct LiveBlock {
    begin: *mut u8,
    next: *mut u8,
    end: *mut u8,
}
impl UsedBlock {
    fn new(memory: *mut u8, size: usize) -> Self {
        return UsedBlock {
            memory: memory,
            size: size,
        };
    }
    unsafe fn destroy<A: OwnedAllocator>(self, allocator: &mut A) {
        allocator.deallocate(self.memory, self.size, arena_heap_alignment());
    }
}
impl LiveBlock {
    fn new() -> Self {
        return LiveBlock {
            begin: null_mut(),
            next: null_mut(),
            end: null_mut(),
        };
    }
    unsafe fn close(self) -> UsedBlock {
        return UsedBlock::new(self.begin, util::distance(self.begin, self.end));
    }
    unsafe fn destroy<A: OwnedAllocator>(&mut self, allocator: &mut A) {
        if !self.begin.is_null() {
            allocator.deallocate(self.begin, util::distance(self.begin, self.end), arena_heap_alignment());
            *self = LiveBlock::new();
        }
    }
    unsafe fn try_ensure_end<A: OwnedAllocator>(&mut self,
                                                allocator: &mut A,
                                                options: &ArenaOptions,
                                                new_end: usize)
                                                -> bool {
        if new_end <= self.end as usize {
            return true;
        } else {
            let needed_size = new_end - (self.begin as usize);
            let mut new_size = (needed_size + 1).next_power_of_two();
            new_size = cmp::min(new_size, options.recommended_max_block_size);
            if new_size < needed_size {
                return false;
            }
            new_size = allocator.usable_size(new_size, arena_heap_alignment());
            let real_new_size = allocator.reallocate_inplace(self.begin,
                                                             util::distance(self.begin, self.end),
                                                             new_size,
                                                             arena_heap_alignment());
            self.end = self.begin.offset(real_new_size as isize);
            return real_new_size == new_size;
        }
    }
    fn initialized(&self) -> bool {
        return !self.begin.is_null();
    }
    unsafe fn initialize<A: OwnedAllocator>(&mut self,
                                            allocator: &mut A,
                                            options: &ArenaOptions,
                                            needed_size: usize,
                                            recommended_size: usize,
                                            align: PowerOfTwo)
                                            -> bool {
        let actual_needed_size = needed_size + align.into() - 1;
        let actual_recommended_size = cmp::min(options.recommended_max_block_size, recommended_size);
        let mut new_size = cmp::max(actual_needed_size, actual_recommended_size);
        new_size = allocator.usable_size(new_size, arena_heap_alignment());
        self.begin = allocator.allocate(new_size, arena_heap_alignment());
        if self.begin.is_null() {
            return false;
        }
        self.next = self.begin;
        self.end = self.next.offset(new_size as isize);
        return true;
    }
    unsafe fn try_allocate<A: OwnedAllocator>(&mut self,
                                              allocator: &mut A,
                                              options: &ArenaOptions,
                                              size: usize,
                                              align: PowerOfTwo)
                                              -> *mut u8 {
        let aligned_next = align.align_size(self.next as usize);
        if self.try_ensure_end(allocator, options, aligned_next + size) {
            self.next = (aligned_next + size) as *mut u8;
            return aligned_next as *mut u8;
        } else {
            return null_mut();
        }
    }
}
pub struct Arena<A: OwnedAllocator> {
    allocator: A,
    options: ArenaOptions,
    used: Vec<UsedBlock>,
    live: LiveBlock,
}
impl<A: OwnedAllocator> Arena<A> {
    pub fn new(allocator: A, options: ArenaOptions) -> Self {
        assert!(options.start_block_size > 0);
        assert!(options.recommended_max_block_size > 0);
        return Arena {
            allocator: allocator,
            options: options,
            used: Vec::new(),
            live: LiveBlock::new(),
        };
    }
}
impl<A: Default + OwnedAllocator> Default for Arena<A> {
    fn default() -> Self {
        return Arena::new(A::default(), ArenaOptions::default());
    }
}
unsafe impl<A: OwnedAllocator> OwnedAllocator for Arena<A> {
    unsafe fn deallocate(&mut self, ptr: *mut u8, old_size: usize, _align: PowerOfTwo) {
        if self.live.next == ptr.offset(old_size as isize) {
            self.live.next = ptr;
        }
    }
    unsafe fn allocate(&mut self, size: usize, align: PowerOfTwo) -> *mut u8 {
        let next_block_size;
        if self.live.initialized() {
            let result = self.live.try_allocate(&mut self.allocator, &self.options, size, align);
            if result.is_null() {
                let old_block = mem::replace(&mut self.live, LiveBlock::new()).close();
                next_block_size = (old_block.size + 1).next_power_of_two();
                self.used.push(old_block);
            } else {
                return result;
            }
        } else {
            next_block_size = self.options.start_block_size;
        }
        if !self.live.initialize(&mut self.allocator, &self.options, size, next_block_size, align) {
            return null_mut();
        }
        let result = self.live.try_allocate(&mut self.allocator, &self.options, size, align);
        assert!(!result.is_null());
        return result;
    }
    unsafe fn reallocate(&mut self, ptr: *mut u8, old_size: usize, new_size: usize, align: PowerOfTwo) -> *mut u8 {
        if self.reallocate_inplace(ptr, old_size, new_size, align) == new_size {
            return ptr;
        } else {
            let ret = self.allocate(new_size, align);
            ptr::copy_nonoverlapping(ptr, ret, old_size);
            return ret;
        }
    }
    unsafe fn reallocate_inplace(&mut self,
                                 ptr: *mut u8,
                                 old_size: usize,
                                 new_size: usize,
                                 _align: PowerOfTwo)
                                 -> usize {
        if self.live.next == ptr.offset(old_size as isize) {
            if self.live.try_ensure_end(&mut self.allocator, &self.options, ptr as usize + new_size) {
                self.live.next = ptr.offset(new_size as isize);
                return new_size;
            }
        }
        return old_size;
    }
    unsafe fn extendable_size(&self, ptr: *mut u8, old_size: usize, _align: PowerOfTwo) -> usize {
        if self.live.next == ptr.offset(old_size as isize) {
            return util::distance(self.live.next, self.live.end);
        } else {
            return 0;
        }
    }

    unsafe fn usable_size(&self, size: usize, _align: PowerOfTwo) -> usize {
        return size;
    }
}
impl<A: OwnedAllocator> Drop for Arena<A> {
    fn drop(&mut self) {
        unsafe {
            for block in self.used.drain(..) {
                block.destroy(&mut self.allocator);
            }
            self.live.destroy(&mut self.allocator);
        }
    }
}
#[test]
fn arena_random_test() {
    unsafe {
        let mut rng = rand::XorShiftRng::new_unseeded();
        for _ in 0..3 {
            let inner_options = Default::default();
            let arena_options = ArenaOptions {
                start_block_size: 1,
                recommended_max_block_size: 256,
            };
            let outer_options = CheckedAllocOptions { ignore_leaks: true, ..Default::default() };
            let mut alloc = CheckedAlloc::new(Arena::new(LoggingAlloc::new(CheckedAlloc::new(HeapAlloc,
                                                                                             inner_options)),
                                                         arena_options),
                                              outer_options);
            for _ in 0..30 {
                let align = PowerOfTwo::new(1 << rng.gen_range(0, 5));
                let mut size = rng.gen_range(1, 64);
                let mut ptr = alloc.allocate(size, align);
                for _ in 0..(*rng.choose(&[0, 1, 5, 100]).unwrap()) {
                    let new_size = rng.gen_range(1, 64);
                    if rng.gen_weighted_bool(2) {
                        let new_ptr = alloc.reallocate(ptr, size, new_size, align);
                        if !new_ptr.is_null() {
                            size = new_size;
                            ptr = new_ptr;
                        }
                    } else {
                        size = alloc.reallocate_inplace(ptr, size, new_size, align);
                    }
                }
                if rng.gen_weighted_bool(2) {
                    alloc.deallocate(ptr, size, align);
                }
            }
        }
    }
}
#[cfg(benchmark)]
const BENCH_COUNT: usize = 1024 * 256;
#[cfg(benchmark)]
fn run_benchmark_alloc<A: OwnedAllocator>(mut a: A) {
    unsafe {
        for i in 0..BENCH_COUNT {
            let ptr: *mut u8 = a.allocate(1, PowerOfTwo::new(1));
            *ptr = 1;
            black_box(ptr);
        }
    }
}
#[inline(never)]
#[cfg(benchmark)]
pub fn run_benchmark_manual() {
    unsafe {
        let align = PowerOfTwo::new(1);
        let buffer = HeapAlloc::default().allocate(BENCH_COUNT, align);
        for i in 0..BENCH_COUNT {
            let ptr = buffer.offset(i as isize);
            *ptr = 1;
            black_box(ptr);
        }
        HeapAlloc::default().deallocate(buffer, BENCH_COUNT, align);
    }
}
// #[inline(never)]
// fn run_benchmark_arena_alloc() {
//    run_benchmark_alloc(Arena::<HeapAlloc>::new(HeapAlloc,
//                                                ArenaOptions {
//                                                    start_block_size: BENCH_COUNT,
//                                                    recommended_max_block_size: BENCH_COUNT,
//                                                    ..ArenaOptions::default()
//                                                }))
// }
// #[bench]
// fn benchmark_arena_alloc(b: &mut Bencher) {
//    b.iter(|| run_benchmark_arena_alloc());
// }
// #[inline(never)]
// fn run_benchmark_block_alloc() {
//    unsafe { run_benchmark_alloc(BlockAlloc::<HeapAlloc>::new(HeapAlloc, BENCH_COUNT)) }
// }
// #[bench]
// fn benchmark_block_alloc(b: &mut Bencher) {
//
//    b.iter(|| run_benchmark_block_alloc());
//
// }
// #[bench]
// fn benchmark_manual(b: &mut Bencher) {
//    b.iter(|| run_benchmark_manual());
// }
