use allocator::Allocator;
use allocator::OwnedAllocator;
use alloc::heap::{deallocate, allocate, reallocate, reallocate_inplace, usable_size};
use allocator::ShareableAllocator;
use util::PowerOfTwo;
pub struct HeapAlloc;
impl Clone for HeapAlloc {
    fn clone(&self) -> Self {
        return HeapAlloc;
    }
}
impl Copy for HeapAlloc {}
impl Default for HeapAlloc {
    fn default() -> Self {
        return HeapAlloc;
    }
}
unsafe impl OwnedAllocator for HeapAlloc {
    unsafe fn allocate(&mut self, new: usize, align: PowerOfTwo) -> *mut u8 {
        return allocate(new, align.into());
    }
    unsafe fn reallocate(&mut self, ptr:*mut u8, old_size:usize, new: usize, align: PowerOfTwo) -> *mut u8 {
        return reallocate(ptr, old_size, new, align.into());
    }
    unsafe fn reallocate_inplace(&mut self, ptr:*mut u8, old_size:usize, new: usize, align: PowerOfTwo) -> usize {
        return reallocate_inplace(ptr, old_size, new, align.into());
    }
    unsafe fn deallocate(&mut self, ptr:*mut u8, old_size:usize, align: PowerOfTwo) {
        return deallocate(ptr,old_size, align.into());
    }
    unsafe fn extendable_size(&self, _ptr:*mut u8, old_size:usize, _align: PowerOfTwo) -> usize {
        return old_size;
    }
    unsafe fn usable_size(&self, size: usize, align: PowerOfTwo) -> usize {
        return usable_size(size, align.into());
    }
}
unsafe impl Allocator for HeapAlloc {}
unsafe impl ShareableAllocator for HeapAlloc {}
