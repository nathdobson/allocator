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
use std::marker::PhantomData;
#[must_use]
pub struct AllocRawBox<T: ?Sized, A: OwnedAllocator> {
    ptr: Unique<T>,
    phantom: PhantomData<*mut A>,
}

impl<T, A: OwnedAllocator> AllocRawBox<T, A> {
    pub fn new(value: T, alloc: &mut A) -> Self {
        unsafe {
            let ptr = if size_of::<T>() == 0 {
                EMPTY as *mut T
            } else {
                let pointer = alloc.allocate(size_of::<T>(), PowerOfTwo::align_of::<T>());
                if pointer.is_null() {
                    oom();
                }
                write(pointer as *mut T, value);
                pointer as *mut T
            };
            return AllocRawBox {
                ptr: Unique::new(ptr),
                phantom: PhantomData,
            };
        }
    }
    pub unsafe fn into_inner(mut self, alloc: &mut A) -> T {
        unsafe {
            let size = size_of_val::<T>(&**self.ptr);
            let align = align_of_val::<T>(&**self.ptr);
            let result = read(*self.ptr);
            if size != 0 {
                alloc.deallocate(*self.ptr as *mut u8, size, PowerOfTwo::new(align));
            }
            mem::forget(self);
            return result;
        }
    }
}
impl<T: ?Sized, A: OwnedAllocator> AllocRawBox<T, A> {
    pub unsafe fn delete(mut self, alloc: &mut A) {
        let size = size_of_val::<T>(&**self.ptr);
        let align = align_of_val::<T>(&**self.ptr);
        drop_in_place::<T>(*self.ptr);
        if size != 0 {
            alloc.deallocate(*self.ptr as *mut u8, size, PowerOfTwo::new(align));
        }
    }
    pub fn into_raw(mut self) -> *mut T {

        let ptr: *mut T = unsafe { self.ptr.get_mut() };
        mem::forget(self);
        return ptr;
    }
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        return AllocRawBox {
            ptr: Unique::new(ptr),
            phantom: PhantomData,
        };
    }
    pub fn get_mut(&mut self) -> *mut T {
        return unsafe { self.ptr.get_mut() };
    }
    pub fn get(&mut self) -> *const T {
        return unsafe { self.ptr.get() };
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, A: OwnedAllocator> CoerceUnsized<AllocRawBox<U, A>> for AllocRawBox<T, A> {}
