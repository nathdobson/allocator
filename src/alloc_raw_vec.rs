// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::mem;
use std::ptr;
use std::slice;
use std::cmp;
use alloc::oom;
use alloc::heap;
use std::ptr::Unique;
use util::PowerOfTwo;
use allocator::OwnedAllocator;
use alloc_box::AllocBox;

pub struct AllocRawVec<T, A: OwnedAllocator> {
    ptr: Unique<T>,
    cap: usize,
    alloc: A,
}

impl<T, A: OwnedAllocator> AllocRawVec<T, A> {
    pub fn new(alloc: A) -> Self {
        unsafe {
            // !0 is usize::MAX. This branch should be stripped at compile time.
            let cap = if mem::size_of::<T>() == 0 { !0 } else { 0 };

            // heap::EMPTY doubles as "unallocated" and "zero-sized allocation"
            AllocRawVec {
                ptr: Unique::new(heap::EMPTY as *mut T),
                cap: cap,
                alloc: alloc,
            }
        }
    }
    pub unsafe fn from_raw_parts(ptr: *mut T, cap: usize, alloc: A) -> Self {
        AllocRawVec {
            ptr: Unique::new(ptr),
            cap: cap,
            alloc: alloc,
        }
    }

    pub fn from_box(mut slice: AllocBox<[T], A>) -> Self {
        unsafe { AllocRawVec::from_raw_parts(slice.as_mut_ptr(), slice.len(), slice.into_allocator()) }
    }
    pub fn ptr(&self) -> *mut T {
        *self.ptr
    }
    pub fn cap(&self) -> usize {
        if mem::size_of::<T>() == 0 {
            !0
        } else {
            self.cap
        }
    }
    pub fn allocator(&self) -> &A {
        return &self.alloc;
    }
    fn amortized_new_size(&self, used_cap: usize, needed_extra_cap: usize) -> (usize, usize) {
        let elem_size = mem::size_of::<T>();
        // Nothing we can really do about these checks :(
        let required_cap = used_cap.checked_add(needed_extra_cap)
            .expect("capacity overflow");
        let mut slack_cap = unsafe {
            self.alloc.extendable_size(self.ptr() as *mut u8, self.cap * elem_size, PowerOfTwo::align_of::<T>()) /
            elem_size
        };
        if slack_cap == self.cap {
            slack_cap = slack_cap * 2;
        }
        let new_cap = cmp::max(slack_cap, required_cap);
        let new_alloc_size = new_cap.checked_mul(elem_size).expect("capacity overflow");
        (new_cap, new_alloc_size)
    }
    pub fn reserve(&mut self, used_cap: usize, needed_extra_cap: usize) {
        unsafe {
            let elem_size = mem::size_of::<T>();

            // NOTE: we don't early branch on ZSTs here because we want this
            // to actually catch "asking for more than usize::MAX" in that case.
            // If we make it past the first branch then we are guaranteed to
            // panic.

            // Don't actually need any more capacity.
            // Wrapping in case they give a bad `used_cap`
            if self.cap().wrapping_sub(used_cap) >= needed_extra_cap {
                return;
            }

            let (new_cap, new_alloc_size) = self.amortized_new_size(used_cap, needed_extra_cap);
            // FIXME: may crash and burn on over-reserve
            alloc_guard(new_alloc_size);

            let ptr = if self.cap == 0 {
                self.alloc.allocate(new_alloc_size, PowerOfTwo::align_of::<T>())
            } else {
                self.alloc
                    .reallocate(*self.ptr as *mut u8, self.cap * elem_size, new_alloc_size, PowerOfTwo::align_of::<T>())
            };

            // If allocate or reallocate fail, we'll get `null` back
            if ptr.is_null() {
                oom()
            }

            self.ptr = Unique::new(ptr as *mut _);
            self.cap = new_cap;
        }
    }
    pub fn reserve_exact(&mut self, used_cap: usize, needed_extra_cap: usize) {
        unsafe {
            let elem_size = mem::size_of::<T>();

            // NOTE: we don't early branch on ZSTs here because we want this
            // to actually catch "asking for more than usize::MAX" in that case.
            // If we make it past the first branch then we are guaranteed to
            // panic.

            // Don't actually need any more capacity.
            // Wrapping in case they gave a bad `used_cap`.
            if self.cap().wrapping_sub(used_cap) >= needed_extra_cap {
                return;
            }

            // Nothing we can really do about these checks :(
            let new_cap = used_cap.checked_add(needed_extra_cap).expect("capacity overflow");
            let new_alloc_size = new_cap.checked_mul(elem_size).expect("capacity overflow");
            alloc_guard(new_alloc_size);

            let ptr = if self.cap == 0 {
                self.alloc.allocate(new_alloc_size, PowerOfTwo::align_of::<T>())
            } else {
                self.alloc
                    .reallocate(self.ptr.get_mut() as *mut T as *mut u8,
                                self.cap * elem_size,
                                new_alloc_size,
                                PowerOfTwo::align_of::<T>())
            };

            // If allocate or reallocate fail, we'll get `null` back
            if ptr.is_null() {
                oom()
            }

            self.ptr = Unique::new(ptr as *mut _);
            self.cap = new_cap;
        }
    }
    pub fn shrink_to_fit(&mut self, amount: usize) {
        let elem_size = mem::size_of::<T>();

        // Set the `cap` because they might be about to promote to a `Box<[T]>`
        if elem_size == 0 {
            self.cap = amount;
            return;
        }

        // This check is my waterloo; it's the only thing Vec wouldn't have to do.
        assert!(self.cap >= amount, "Tried to shrink to a larger capacity");

        if amount == 0 {
            unimplemented!();
        } else if self.cap != amount {
            unsafe {
                // Overflow check is unnecessary as the vector is already at
                // least this large.
                let ptr = self.alloc
                    .reallocate(*self.ptr as *mut u8,
                                self.cap * elem_size,
                                amount * elem_size,
                                PowerOfTwo::align_of::<T>());
                if ptr.is_null() {
                    oom()
                }
                self.ptr = Unique::new(ptr as *mut _);
            }
            self.cap = amount;
        }
    }
    pub unsafe fn into_box(mut self) -> AllocBox<[T], A> {
        // NOTE: not calling `cap()` here, actually using the real `cap` field!
        let slice: *mut [T] = slice::from_raw_parts_mut(self.ptr(), self.cap);
        let output: AllocBox<[T], A> = AllocBox::from_raw_parts(slice, ptr::read(&mut self.alloc as *mut A));
        mem::forget(self);
        output
    }
}

impl<T, A: OwnedAllocator> Drop for AllocRawVec<T, A> {
    fn drop(&mut self) {
        let elem_size = mem::size_of::<T>();
        if elem_size != 0 && self.cap != 0 {
            let num_bytes = elem_size * self.cap;
            unsafe {
                self.alloc.deallocate(*self.ptr as *mut _, num_bytes, PowerOfTwo::align_of::<T>());
            }
        }
    }
}



// We need to guarantee the following:
// * We don't ever allocate `> isize::MAX` byte-size objects
// * We don't overflow `usize::MAX` and actually allocate too little
//
// On 64-bit we just need to check for overflow since trying to allocate
// `> isize::MAX` bytes will surely fail. On 32-bit we need to add an extra
// guard for this in case we're running on a platform which can use all 4GB in
// user-space. e.g. PAE or x32

#[inline]
fn alloc_guard(alloc_size: usize) {
    if mem::size_of::<usize>() < 8 {
        assert!(alloc_size <= ::core::isize::MAX as usize, "capacity overflow");
    }
}
