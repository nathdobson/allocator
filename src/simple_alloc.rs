use allocator::OwnedAllocator;
use util::PowerOfTwo;
use std::mem::size_of;
use std::ptr::null_mut;
use allocator::ShareableAllocator;
use std::ptr::write;
pub enum MockAllocResult {
    Allocate(*mut u8),
    Reallocate(*mut u8),
    ReallocateInplace(usize),
    Deallocate,
}
pub struct MockAlloc {
    schedule: Vec<MockAllocResult>,
}
impl MockAlloc {
    pub unsafe fn new(schedule: Vec<MockAllocResult>) -> Self {
        return MockAlloc { schedule: schedule };
    }
}
unsafe impl OwnedAllocator for MockAlloc {
    unsafe fn deallocate(&mut self, _ptr: *mut u8, _old_size: usize, _align: PowerOfTwo) {
        match self.schedule.remove(0) {
            MockAllocResult::Deallocate => (),
            _ => panic!(),
        }
    }
    unsafe fn allocate(&mut self, _size: usize, _align: PowerOfTwo) -> *mut u8 {
        match self.schedule.remove(0) {
            MockAllocResult::Allocate(ptr) => ptr,
            _ => panic!(),
        }
    }
    unsafe fn reallocate(&mut self, _ptr: *mut u8, _old_size: usize, _size: usize, _align: PowerOfTwo) -> *mut u8 {
        match self.schedule.remove(0) {
            MockAllocResult::Reallocate(ptr) => ptr,
            _ => panic!(),
        }
    }
    unsafe fn reallocate_inplace(&mut self, _ptr: *mut u8, _old_size: usize, _size: usize, _align: PowerOfTwo) -> usize {
        match self.schedule.remove(0) {
            MockAllocResult::ReallocateInplace(size) => size,
            _ => panic!(),
        }
    }
}
unsafe impl ShareableAllocator for MockAlloc {}
pub struct DeadBeefAllocator<A: OwnedAllocator> {
    allocator: A,
    beef: Vec<u8>,
}
impl<A: OwnedAllocator> DeadBeefAllocator<A> {
    fn new(allocator: A, beef: Vec<u8>) -> Self {
        return DeadBeefAllocator {
            allocator: allocator,
            beef: beef,
        };
    }
    unsafe fn fill(&self, ptr: *mut u8, size: usize) {
        for i in 0..(size as usize) {
            write(ptr.offset(i as isize), self.beef[i % self.beef.len()]);
        }
    }
}
impl<A: Default + OwnedAllocator> Default for DeadBeefAllocator<A> {
    fn default() -> Self {
        let mut beef = Vec::new();
        while beef.len() < size_of::<usize>() {
            beef.push(0xDE);
            beef.push(0xAD);
            beef.push(0xBE);
            beef.push(0xEF);
        }
        beef.resize(size_of::<usize>(), 0);
        return Self::new(A::default(), beef);
    }
}
unsafe impl<A: OwnedAllocator> OwnedAllocator for DeadBeefAllocator<A> {
    unsafe fn deallocate(&mut self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) {
        self.fill(ptr, self.allocator.usable_size(old_size, align));
        return self.allocator.deallocate(ptr, old_size, align);
    }
    unsafe fn allocate(&mut self, size: usize, align: PowerOfTwo) -> *mut u8 {
        let ret = self.allocator.allocate(size, align);
        if !ret.is_null() {
            self.fill(ret, self.allocator.usable_size(size, align));
        }
        return ret;
    }
    unsafe fn reallocate(&mut self, ptr: *mut u8, old_size: usize, size: usize, align: PowerOfTwo) -> *mut u8 {
        let ret = self.allocator.reallocate(ptr, old_size, size, align);
        if !ret.is_null() && size > old_size {
            self.fill(ret.offset(old_size as isize), size - old_size);
        }
        return ret;
    }
    unsafe fn reallocate_inplace(&mut self, ptr: *mut u8, old_size: usize, size: usize, align: PowerOfTwo) -> usize {
        let ret = self.allocator.reallocate_inplace(ptr, old_size, size, align);
        if ret > old_size {
            self.fill(ptr.offset(old_size as isize), ret - old_size);
        }
        return ret;
    }
    unsafe fn extendable_size(&self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) -> usize {
        return self.allocator.extendable_size(ptr, old_size, align);
    }

    unsafe fn usable_size(&self, size: usize, align: PowerOfTwo) -> usize {
        return self.allocator.usable_size(size, align);
    }
}
unsafe impl<A: ShareableAllocator> ShareableAllocator for DeadBeefAllocator<A> {}
pub struct LoggingAlloc<A: OwnedAllocator> {
    allocator: A,
}
impl<A: OwnedAllocator> LoggingAlloc<A> {
    pub fn new(allocator: A) -> Self {
        return LoggingAlloc { allocator: allocator };
    }
}
impl<A: Default + OwnedAllocator> Default for LoggingAlloc<A> {
    fn default() -> Self {
        return Self::new(Default::default());
    }
}
unsafe impl<A: OwnedAllocator> OwnedAllocator for LoggingAlloc<A> {
    unsafe fn deallocate(&mut self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) {
        println!("deallocate(0x{:x}, {}, {})", ptr as usize, old_size, align);
        self.allocator.deallocate(ptr, old_size, align);
    }
    unsafe fn allocate(&mut self, size: usize, align: PowerOfTwo) -> *mut u8 {
        println!("allocate({}, {})", size, align);
        let ret = self.allocator.allocate(size, align);
        println!("allocate -> 0x{:x}", ret as usize);
        return ret;
    }
    unsafe fn reallocate(&mut self, ptr: *mut u8, old_size: usize, size: usize, align: PowerOfTwo) -> *mut u8 {
        println!("reallocate(0x{:x}, {}, {}, {})", ptr as usize, old_size, size, align);
        let ret = self.allocator.reallocate(ptr, old_size, size, align);
        println!("reallocate -> 0x{:x}", ret as usize);
        return ret;
    }
    unsafe fn reallocate_inplace(&mut self, ptr: *mut u8, old_size: usize, size: usize, align: PowerOfTwo) -> usize {
        println!("reallocate_inplace(0x{:x}, {}, {}, {})", ptr as usize, old_size, size, align);
        let ret = self.allocator.reallocate_inplace(ptr, old_size, size, align);
        println!("reallocate_inplace -> {}", ret);
        return ret;
    }
    unsafe fn extendable_size(&self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) -> usize {
        return self.allocator.extendable_size(ptr, old_size, align);
    }

    unsafe fn usable_size(&self, size: usize, align: PowerOfTwo) -> usize {
        return self.allocator.usable_size(size, align);
    }
}
unsafe impl<A: ShareableAllocator> ShareableAllocator for LoggingAlloc<A> {}
pub struct BlockAlloc<A: OwnedAllocator> {
    allocator: A,
    block: *mut u8,
    next: *mut u8,
    size: usize,
}
impl<A: OwnedAllocator> BlockAlloc<A> {
    pub unsafe fn new(mut allocator: A, size: usize) -> Self {
        assert!(size > 0);
        let block = allocator.allocate(size, PowerOfTwo::new(1));
        return BlockAlloc {
            allocator: allocator,
            block: block,
            next: block,
            size: size,
        };
    }
}
unsafe impl<A: OwnedAllocator> OwnedAllocator for BlockAlloc<A> {
    unsafe fn deallocate(&mut self, _ptr: *mut u8, _old_size: usize, _align: PowerOfTwo) {}
    unsafe fn allocate(&mut self, size: usize, _align: PowerOfTwo) -> *mut u8 {
        let ret = self.next;
        self.next = self.next.offset(size as isize);
        return ret;
    }
    unsafe fn reallocate(&mut self, _ptr: *mut u8, _old_size: usize, _size: usize, _align: PowerOfTwo) -> *mut u8 {
        return null_mut();
    }
    unsafe fn reallocate_inplace(&mut self, _ptr: *mut u8, old_size: usize, _size: usize, _align: PowerOfTwo) -> usize {
        return old_size;
    }
}
impl<A: OwnedAllocator> Drop for BlockAlloc<A> {
    fn drop(&mut self) {
        unsafe {
            self.allocator.deallocate(self.block, self.size, PowerOfTwo::new(1));
        }
    }
}
unsafe impl<A: ShareableAllocator> ShareableAllocator for BlockAlloc<A> {}
