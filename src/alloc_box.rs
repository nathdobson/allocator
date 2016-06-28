use core::any::Any;
use core::borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{self, Hash};
use core::mem;
use std::ops::Deref;
use std::ops::DerefMut;

use core::ptr::{self, Unique};
use core::convert::From;
use allocator::Allocator;
use allocator::OwnedAllocator;
use std::mem::size_of;
use std::mem::align_of;
use std::ptr::write;
use std::intrinsics::drop_in_place;
use alloc::oom::oom;
use util::PowerOfTwo;
use heap_alloc::HeapAlloc;
use checked_alloc::CheckedAlloc;
use alloc::heap::EMPTY;
use core::mem::{align_of_val, size_of_val};
use allocator::SharedAlloc;
use std::marker::Unsize;
use std::ops::CoerceUnsized;
use util::CheckDrop;
use std::marker::Reflect;
use std::rc::Rc;
use std::convert::AsMut;
use std::convert::AsRef;
use std::ptr::read;
use std::mem::forget;
use alloc_raw_box::AllocRawBox;

pub struct AllocBox<T: ?Sized, A: OwnedAllocator> {
    alloc: A,
    ptr: AllocRawBox<T, A>,
}

impl<T, A: OwnedAllocator> AllocBox<T, A> {
    pub fn new(x: T, mut alloc: A) -> Self {
        unsafe {
            let ptr = AllocRawBox::new(x, &mut alloc);
            return AllocBox {
                ptr: ptr,
                alloc: alloc,
            };
        }
    }
    pub fn into_inner(mut self) -> T {
        unsafe {
            return self.ptr.into_inner(&mut self.alloc);
        }
    }
    pub fn into_inner_with_allocator(mut self) -> (T, A) {
        unsafe {
            return (self.ptr.into_inner(&mut self.alloc), self.alloc);
        }
    }
}
impl<T: ?Sized, A: OwnedAllocator> AllocBox<T, A> {
    // Unsafe because caller must deallocate, not because of undefined behavior.
    pub unsafe fn into_raw_parts(mut self) -> (A, *mut T) {
        let alloc: A = read(&mut self.alloc as *mut A);
        let ptr: *mut T = self.ptr.get_mut();
        mem::forget(self);
        return (alloc, ptr);
    }
    pub fn into_allocator(self) -> A {
        unsafe {
            self.ptr.delete(&mut self.alloc);
            return self.alloc;
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


impl<T: ?Sized, A: OwnedAllocator> Deref for AllocBox<T, A> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr.get() }
    }
}
impl<T: ?Sized, A: OwnedAllocator> DerefMut for AllocBox<T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut*self.ptr.get_mut() }
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
            self.ptr.delete(&mut self.alloc);
        }
    }
}
impl<T: ?Sized + Unsize<U>, U: ?Sized, A: OwnedAllocator> CoerceUnsized<AllocBox<U, A>> for AllocBox<T, A> {}

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
        fn must_drop<'a, T: Reflect + 'a>(x: T) {
            let allocator: SharedAlloc<CheckedAlloc<HeapAlloc>> = Default::default();
            let b: AllocBox<T, _> = AllocBox::<T, _>::new(x, &allocator);
            let b2: AllocBox<Reflect + 'a, _> = b;
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
        let (_, v) = b.into_inner_with_allocator();
        assert_eq!(12, v);
    }
}
