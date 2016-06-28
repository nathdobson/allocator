use std::cell::UnsafeCell;
use util::PowerOfTwo;
// Intended allocator implementations:
// system allocator
// c malloc/free
// jemalloc
// arena
// freelist
//
// Intended allocator clients:
//
// Box
// allocate exactly x bytes
//
// Vec
// allocate some bytes
// reallocate extendable_size if possible, otherwise double
//
// HashMap
// allocate/reallocate powers of two
//
// Arena
// allocate/reallocate_inplace at least x bytes
//
// PassthroughFreeList
// allocate many buffers of an exact size
//
// BitVectorFreeList
// allocate one buffer and an "is free" bitvector
//
//
// pub unsafe trait BoxAllocator{
//    unsafe fn allocate_box(&mut self,size:usize,align:PowerOfTwo) -> *mut u8;
//    unsafe fn deallocate_box(&mut self,ptr:*mut u8,size:usize,align:PowerOfTwo) -> *mut u8;
// }
// struct AllocRawVec{
//    unsafe fn allocate_vec(&mut self,count:usize,size:usize,align:PowerOfTwo) -> *mut u8;
//    unsafe fn
// }
// pub unsafe trait VecAllocator{
//    unsafe fn allocate(&mut self,
// }
pub unsafe trait OwnedAllocator {
    unsafe fn allocate(&mut self, new: usize, align: PowerOfTwo) -> *mut u8;
    unsafe fn reallocate(&mut self, ptr: *mut u8, old_size: usize, new_size: usize, align: PowerOfTwo) -> *mut u8;
    unsafe fn reallocate_inplace(&mut self,
                                 ptr: *mut u8,
                                 old_size: usize,
                                 new_size: usize,
                                 align: PowerOfTwo)
                                 -> usize;
    unsafe fn deallocate(&mut self, ptr: *mut u8, old_size: usize, align: PowerOfTwo);
    unsafe fn extendable_size(&self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) -> usize {
        let _ = ptr;
        let _ = align;
        return old_size;
    }
    unsafe fn usable_size(&self, size: usize, align: PowerOfTwo) -> usize {
        let _ = align;
        return size;
    }
}
pub unsafe trait Allocator: OwnedAllocator + Copy {}
pub unsafe trait ShareableAllocator: OwnedAllocator {
    // indicates that this allocator may be used from a SharedAlloc
}
pub struct SharedAlloc<A: ShareableAllocator>(UnsafeCell<A>);
impl<A> SharedAlloc<A>
    where A: ShareableAllocator
{
    pub fn new(allocator: A) -> Self {
        return SharedAlloc(UnsafeCell::new(allocator));
    }
    pub fn get(&self) -> *mut A {
        return self.0.get();
    }
}
impl<A> Default for SharedAlloc<A>
    where A: Default + ShareableAllocator
{
    fn default() -> Self {
        return Self::new(Default::default());
    }
}
unsafe impl<'a, A> OwnedAllocator for &'a SharedAlloc<A>
    where A: ShareableAllocator
{
    unsafe fn allocate(&mut self, new: usize, align: PowerOfTwo) -> *mut u8 {
        return (*self.get()).allocate(new, align);
    }
    unsafe fn reallocate(&mut self, ptr: *mut u8, old_size: usize, new_size: usize, align: PowerOfTwo) -> *mut u8 {
        return (*self.get()).reallocate(ptr, old_size, new_size, align);
    }
    unsafe fn reallocate_inplace(&mut self,
                                 ptr: *mut u8,
                                 old_size: usize,
                                 new_size: usize,
                                 align: PowerOfTwo)
                                 -> usize {
        return (*self.get()).reallocate_inplace(ptr, old_size, new_size, align);

    }
    unsafe fn deallocate(&mut self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) {
        return (*self.get()).deallocate(ptr, old_size, align);
    }
    unsafe fn extendable_size(&self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) -> usize {
        return (*self.get()).extendable_size(ptr, old_size, align);
    }
    unsafe fn usable_size(&self, size: usize, align: PowerOfTwo) -> usize {
        return (*self.get()).usable_size(size, align);
    }
}
unsafe impl<'a, A> Allocator for &'a SharedAlloc<A> where A: ShareableAllocator {}
