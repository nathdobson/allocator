use std::borrow;
use std::mem;
use std::marker;
use std::ptr;
use std::ops;
use allocator::Allocator;
use allocator::OwnedAllocator;
use alloc_raw_box::AllocRawBox;
#[cfg(test)]
use allocator::SharedAlloc;
#[cfg(test)]
use checked_alloc::CheckedAlloc;
#[cfg(test)]
use heap_alloc::HeapAlloc;
#[cfg(test)]
use util::CheckDrop;

pub struct AllocBox<T: ?Sized, A: OwnedAllocator> {
    alloc: A,
    ptr: AllocRawBox<T, A>,
}

impl<T, A: OwnedAllocator> AllocBox<T, A> {
    pub fn new(x: T, mut alloc: A) -> Self {
        let ptr = AllocRawBox::new(x, &mut alloc);
        return AllocBox {
            ptr: ptr,
            alloc: alloc,
        };
    }
    pub fn into_inner(self) -> T {
        let (mut alloc, raw_box) = self.into_raw_parts();
        unsafe { raw_box.into_inner(&mut alloc) }
    }
    pub fn into_inner_with_allocator(self) -> (T, A) {
        let (mut alloc, raw_box) = self.into_raw_parts();
        unsafe { (raw_box.into_inner(&mut alloc), alloc) }
    }
}
impl<T: ?Sized, A: OwnedAllocator> AllocBox<T, A> {
    pub fn into_raw_parts(mut self) -> (A, AllocRawBox<T, A>) {
        unsafe {
            let alloc: A = ptr::read(&mut self.alloc as *mut A);
            let ptr: AllocRawBox<T, A> = ptr::read(&mut self.ptr as *mut AllocRawBox<T, A>);
            mem::forget(self);
            return (alloc, ptr);
        }
    }
    pub fn into_allocator(self) -> A {
        unsafe {
            let (mut alloc, raw_box) = self.into_raw_parts();
            raw_box.delete(&mut alloc);
            return alloc;
        }
    }
    pub unsafe fn from_raw_parts(ptr: *mut T, alloc: A) -> Self {
        return AllocBox {
            alloc: alloc,
            ptr: AllocRawBox::from_raw(ptr),
        };
    }
}

impl<T: Default, A: Default + Allocator> Default for AllocBox<T, A> {
    fn default() -> Self {
        return Self::new(Default::default(), Default::default());
    }
}


impl<T: ?Sized, A: OwnedAllocator> ops::Deref for AllocBox<T, A> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr.get() }
    }
}
impl<T: ?Sized, A: OwnedAllocator> ops::DerefMut for AllocBox<T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr.get_mut() }
    }
}

impl<T: ?Sized, A: OwnedAllocator> borrow::Borrow<T> for AllocBox<T, A> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, A: OwnedAllocator> borrow::BorrowMut<T> for AllocBox<T, A> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized, A: OwnedAllocator> AsRef<T> for AllocBox<T, A> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, A: OwnedAllocator> AsMut<T> for AllocBox<T, A> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}


impl<T: ?Sized, A: OwnedAllocator> Drop for AllocBox<T, A> {
    fn drop(&mut self) {
        unsafe {
            mem::replace(&mut self.ptr, mem::uninitialized() /* well then... */).delete(&mut self.alloc);
        }
    }
}
impl<T: ?Sized + marker::Unsize<U>, U: ?Sized, A: OwnedAllocator> ops::CoerceUnsized<AllocBox<U, A>> for AllocBox<T, A> {}

#[test]
fn box_on_heap_test() {
    let allocator: SharedAlloc<CheckedAlloc<HeapAlloc>> = Default::default();
    {
        let b = AllocBox::new(12i32, &allocator);
        assert_eq!(12, *b);
    }
    {
        let mut d = CheckDrop::new();
        let b: AllocBox<[_], _> = AllocBox::<[_; 1], _>::new([d.build()], &allocator);
        assert_eq!(1, (*b).len());
    }
    {
        let mut d = CheckDrop::new();
        #[allow(unused_variables)]
        fn must_drop<'a, T: marker::Reflect + 'a>(x: T) {
            let allocator: SharedAlloc<CheckedAlloc<HeapAlloc>> = Default::default();
            let b: AllocBox<T, _> = AllocBox::<T, _>::new(x, &allocator);
            let b2: AllocBox<marker::Reflect + 'a, _> = b;
            
        }
        must_drop(d.build());
    }
    {
        let b = AllocBox::new(12i32, &allocator);
        let v = b.into_inner();
        assert_eq!(12, v);
    }
    {
        let b = AllocBox::new(12i32, &allocator);
        b.into_allocator();
    }
    {
        let b = AllocBox::new(12i32, &allocator);
        let (v, _) = b.into_inner_with_allocator();
        assert_eq!(12, v);
    }
}
