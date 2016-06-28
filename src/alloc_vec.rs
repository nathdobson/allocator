// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::heap;
use std::cmp;
use std::fmt;
use std::hash::{self, Hash};
use std::intrinsics;
use std::iter;
use std::mem;
use std::ops::{self, Index, IndexMut};
use std::ptr;
use std::slice;
use collections::range::RangeArgument;
use alloc_raw_vec::AllocRawVec;
use allocator::OwnedAllocator;
use allocator::Allocator;
use alloc_box::AllocBox;
pub struct AllocVec<T, A: OwnedAllocator> {
    buf: AllocRawVec<T, A>,
    len: usize,
}

impl<T, A: OwnedAllocator> AllocVec<T, A> {
    pub fn new() -> Self
        where A: Default
    {
        AllocVec {
            buf: AllocRawVec::new(Default::default()),
            len: 0,
        }
    }
    pub fn with_allocator(alloc: A) -> Self {
        AllocVec {
            buf: AllocRawVec::new(alloc),
            len: 0,
        }
    }
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize, alloc: A) -> Self {
        AllocVec {
            buf: AllocRawVec::from_raw_parts(ptr, capacity, alloc),
            len: length,
        }
    }
    pub fn capacity(&self) -> usize {
        self.buf.cap()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(self.len, additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.buf.reserve_exact(self.len, additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.buf.shrink_to_fit(self.len);
    }

    pub fn into_boxed_slice(mut self) -> AllocBox<[T], A> {
        unsafe {
            self.shrink_to_fit();
            let buf = ptr::read(&self.buf);
            mem::forget(self);
            buf.into_box()
        }
    }

    pub fn truncate(&mut self, len: usize) {
        unsafe {
            // drop any extra elements
            while len < self.len {
                // decrement len before the drop_in_place(), so a panic on Drop
                // doesn't re-drop the just-failed value.
                self.len -= 1;
                let len = self.len;
                ptr::drop_in_place(self.get_unchecked_mut(len));
            }
        }
    }

    pub fn as_slice(&self) -> &[T] {
        self
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self[..]
    }

    pub unsafe fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        let length = self.len();
        self.swap(index, length - 1);
        self.pop().unwrap()
    }

    pub fn insert(&mut self, index: usize, element: T) {
        let len = self.len();
        assert!(index <= len);

        // space for the new element
        if len == self.buf.cap() {
            self.buf.reserve(len, 1);
        }

        unsafe {
            // infallible
            // The spot to put the new value
            {
                let p = self.as_mut_ptr().offset(index as isize);
                // Shift everything over to make space. (Duplicating the
                // `index`th element into two consecutive places.)
                ptr::copy(p, p.offset(1), len - index);
                // Write it in, overwriting the first copy of the `index`th
                // element.
                ptr::write(p, element);
            }
            self.set_len(len + 1);
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len();
        assert!(index < len);
        unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.as_mut_ptr().offset(index as isize);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the vector at the same time.
                ret = ptr::read(ptr);

                // Shift everything down to fill in that spot.
                ptr::copy(ptr.offset(1), ptr, len - index - 1);
            }
            self.set_len(len - 1);
            ret
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
        where F: FnMut(&T) -> bool
    {
        let len = self.len();
        let mut del = 0;
        {
            let v = &mut **self;

            for i in 0..len {
                if !f(&v[i]) {
                    del += 1;
                } else if del > 0 {
                    v.swap(i - del, i);
                }
            }
        }
        if del > 0 {
            self.truncate(len - del);
        }
    }

    pub fn push(&mut self, value: T) {
        // This will panic or abort if we would allocate > isize::MAX bytes
        // or if the length increment would overflow for zero-sized types.
        if self.len == self.buf.cap() {
            self.buf.reserve(self.len, 1);
        }
        unsafe {
            let end = self.as_mut_ptr().offset(self.len as isize);
            ptr::write(end, value);
            self.len += 1;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                self.len -= 1;
                Some(ptr::read(self.get_unchecked(self.len())))
            }
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        self.reserve(other.len());
        let len = self.len();
        unsafe {
            ptr::copy_nonoverlapping(other.as_ptr(), self.get_unchecked_mut(len), other.len());
        }

        self.len += other.len();
        unsafe {
            other.set_len(0);
        }
    }

    pub fn drain<R>(&mut self, range: R) -> AllocDrain<T, A>
        where R: RangeArgument<usize>
    {
        // Memory safety
        //
        // When the Drain is first created, it shortens the length of
        // the source vector to make sure no uninitalized or moved-from elements
        // are accessible at all if the Drain's destructor never gets to run.
        //
        // Drain will ptr::read out the values to remove.
        // When finished, remaining tail of the vec is copied back to cover
        // the hole, and the vector length is restored to the new length.
        //
        let len = self.len();
        let start = *range.start().unwrap_or(&0);
        let end = *range.end().unwrap_or(&len);
        assert!(start <= end);
        assert!(end <= len);

        unsafe {
            // set self.vec length's to start, to be safe in case Drain is leaked
            self.set_len(start);
            // Use the borrow in the IterMut to indicate borrowing behavior of the
            // whole Drain iterator (like &mut T).
            let range_slice = slice::from_raw_parts_mut(self.as_mut_ptr().offset(start as isize), end - start);
            AllocDrain {
                tail_start: end,
                tail_len: len - end,
                iter: range_slice.iter_mut(),
                vec: self as *mut _,
            }
        }
    }

    pub fn clear(&mut self) {
        self.truncate(0)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Clone, A: OwnedAllocator> AllocVec<T, A> {
    pub fn resize(&mut self, new_len: usize, value: T) {
        let len = self.len();

        if new_len > len {
            self.extend_with_element(new_len - len, value);
        } else {
            self.truncate(new_len);
        }
    }

    /// Extend the vector by `n` additional clones of `value`.
    fn extend_with_element(&mut self, n: usize, value: T) {
        self.reserve(n);

        unsafe {
            let len = self.len();
            let mut ptr = self.as_mut_ptr().offset(len as isize);
            // Write all elements except the last one
            for i in 1..n {
                ptr::write(ptr, value.clone());
                ptr = ptr.offset(1);
                // Increment the length in every step in case clone() panics
                self.set_len(len + i);
            }

            if n > 0 {
                // We can write the last element directly without cloning needlessly
                ptr::write(ptr, value);
                self.set_len(len + n);
            }
        }
    }


    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.reserve(other.len());

        for i in 0..other.len() {
            let len = self.len();

            // Unsafe code so this can be optimised to a memcpy (or something
            // similarly fast) when T is Copy. LLVM is easily confused, so any
            // extra operations during the loop can prevent this optimisation.
            unsafe {
                ptr::write(self.get_unchecked_mut(len), other.get_unchecked(i).clone());
                self.set_len(len + 1);
            }
        }
    }
}

impl<T: PartialEq, A: OwnedAllocator> AllocVec<T, A> {
    pub fn dedup(&mut self) {
        unsafe {
            // Although we have a mutable reference to `self`, we cannot make
            // *arbitrary* changes. The `PartialEq` comparisons could panic, so we
            // must ensure that the vector is in a valid state at all time.
            //
            // The way that we handle this is by using swaps; we iterate
            // over all the elements, swapping as we go so that at the end
            // the elements we wish to keep are in the front, and those we
            // wish to reject are at the back. We can then truncate the
            // vector. This operation is still O(n).
            //
            // Example: We start in this state, where `r` represents "next
            // read" and `w` represents "next_write`.
            //
            //           r
            //     +---+---+---+---+---+---+
            //     | 0 | 1 | 1 | 2 | 3 | 3 |
            //     +---+---+---+---+---+---+
            //           w
            //
            // Comparing self[r] against self[w-1], this is not a duplicate, so
            // we swap self[r] and self[w] (no effect as r==w) and then increment both
            // r and w, leaving us with:
            //
            //               r
            //     +---+---+---+---+---+---+
            //     | 0 | 1 | 1 | 2 | 3 | 3 |
            //     +---+---+---+---+---+---+
            //               w
            //
            // Comparing self[r] against self[w-1], this value is a duplicate,
            // so we increment `r` but leave everything else unchanged:
            //
            //                   r
            //     +---+---+---+---+---+---+
            //     | 0 | 1 | 1 | 2 | 3 | 3 |
            //     +---+---+---+---+---+---+
            //               w
            //
            // Comparing self[r] against self[w-1], this is not a duplicate,
            // so swap self[r] and self[w] and advance r and w:
            //
            //                       r
            //     +---+---+---+---+---+---+
            //     | 0 | 1 | 2 | 1 | 3 | 3 |
            //     +---+---+---+---+---+---+
            //                   w
            //
            // Not a duplicate, repeat:
            //
            //                           r
            //     +---+---+---+---+---+---+
            //     | 0 | 1 | 2 | 3 | 1 | 3 |
            //     +---+---+---+---+---+---+
            //                       w
            //
            // Duplicate, advance r. End of vec. Truncate to w.

            let ln = self.len();
            if ln <= 1 {
                return;
            }

            // Avoid bounds checks by using raw pointers.
            let p = self.as_mut_ptr();
            let mut r: usize = 1;
            let mut w: usize = 1;

            while r < ln {
                let p_r = p.offset(r as isize);
                let p_wm1 = p.offset((w - 1) as isize);
                if *p_r != *p_wm1 {
                    if r != w {
                        let p_w = p_wm1.offset(1);
                        mem::swap(&mut *p_r, &mut *p_w);
                    }
                    w += 1;
                }
                r += 1;
            }

            self.truncate(w);
        }
    }
}

/// /////////////////////////////////////////////////////////////////////////////
/// Internal methods and functions
/// /////////////////////////////////////////////////////////////////////////////
#[doc(hidden)]
pub fn from_elem<T: Clone, A: OwnedAllocator>(elem: T, n: usize, allocator: A) -> AllocVec<T, A> {
    let mut v = AllocVec::with_allocator(allocator);
    v.reserve_exact(n);
    v.extend_with_element(n, elem);
    v
}

/// /////////////////////////////////////////////////////////////////////////////
/// Common trait implementations for Vec
/// /////////////////////////////////////////////////////////////////////////////
impl<T: Clone, A: Allocator> Clone for AllocVec<T, A> {
    fn clone(&self) -> AllocVec<T, A> {
        let mut ret = AllocVec::with_allocator(*self.buf.allocator());
        ret.extend_from_slice(self.as_slice());
        return ret;
    }


    fn clone_from(&mut self, other: &AllocVec<T, A>) {
        // drop anything in self that will not be overwritten
        self.truncate(other.len());
        let len = self.len();

        // reuse the contained values' allocations/resources.
        self.clone_from_slice(&other[..len]);

        // self.len <= other.len due to the truncate above, so the
        // slice here is always in-bounds.
        self.extend_from_slice(&other[len..]);
    }
}
impl<T: Hash, A: OwnedAllocator> Hash for AllocVec<T, A> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}
impl<T, A: OwnedAllocator> Index<usize> for AllocVec<T, A> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &T {
        // NB built-in indexing via `&[T]`
        &(**self)[index]
    }
}
impl<T, A: OwnedAllocator> IndexMut<usize> for AllocVec<T, A> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        // NB built-in indexing via `&mut [T]`
        &mut (**self)[index]
    }
}

impl<T, A: OwnedAllocator> ops::Index<ops::Range<usize>> for AllocVec<T, A> {
    type Output = [T];

    #[inline]
    fn index(&self, index: ops::Range<usize>) -> &[T] {
        Index::index(&**self, index)
    }
}
impl<T, A: OwnedAllocator> ops::Index<ops::RangeTo<usize>> for AllocVec<T, A> {
    type Output = [T];

    #[inline]
    fn index(&self, index: ops::RangeTo<usize>) -> &[T] {
        Index::index(&**self, index)
    }
}
impl<T, A: OwnedAllocator> ops::Index<ops::RangeFrom<usize>> for AllocVec<T, A> {
    type Output = [T];

    #[inline]
    fn index(&self, index: ops::RangeFrom<usize>) -> &[T] {
        Index::index(&**self, index)
    }
}
impl<T, A: OwnedAllocator> ops::Index<ops::RangeFull> for AllocVec<T, A> {
    type Output = [T];

    #[inline]
    fn index(&self, _index: ops::RangeFull) -> &[T] {
        self
    }
}
impl<T, A: OwnedAllocator> ops::IndexMut<ops::Range<usize>> for AllocVec<T, A> {
    #[inline]
    fn index_mut(&mut self, index: ops::Range<usize>) -> &mut [T] {
        IndexMut::index_mut(&mut **self, index)
    }
}
impl<T, A: OwnedAllocator> ops::IndexMut<ops::RangeTo<usize>> for AllocVec<T, A> {
    #[inline]
    fn index_mut(&mut self, index: ops::RangeTo<usize>) -> &mut [T] {
        IndexMut::index_mut(&mut **self, index)
    }
}
impl<T, A: OwnedAllocator> ops::IndexMut<ops::RangeFrom<usize>> for AllocVec<T, A> {
    #[inline]
    fn index_mut(&mut self, index: ops::RangeFrom<usize>) -> &mut [T] {
        IndexMut::index_mut(&mut **self, index)
    }
}
impl<T, A: OwnedAllocator> ops::IndexMut<ops::RangeFull> for AllocVec<T, A> {
    #[inline]
    fn index_mut(&mut self, _index: ops::RangeFull) -> &mut [T] {
        self
    }
}
impl<T, A: OwnedAllocator> ops::Deref for AllocVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            let p = self.buf.ptr();
            intrinsics::assume(!p.is_null());
            slice::from_raw_parts(p, self.len)
        }
    }
}
impl<T, A: OwnedAllocator> ops::DerefMut for AllocVec<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            let ptr = self.buf.ptr();
            intrinsics::assume(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, self.len)
        }
    }
}
impl<T, A: OwnedAllocator + Default> iter::FromIterator<T> for AllocVec<T, A> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> AllocVec<T, A> {
        // Unroll the first iteration, as the vector is going to be
        // expanded on this iteration in every case when the iterable is not
        // empty, but the loop in extend_desugared() is not going to see the
        // vector being full in the few subsequent loop iterations.
        // So we get better branch prediction.
        let mut iterator = iter.into_iter();
        let mut vector = match iterator.next() {
            None => return AllocVec::new(),
            Some(element) => {
                let (lower, _) = iterator.size_hint();
                let mut vector = AllocVec::new();
                vector.reserve_exact(lower.saturating_add(1));
                unsafe {
                    ptr::write(vector.get_unchecked_mut(0), element);
                    vector.set_len(1);
                }
                vector
            }
        };
        vector.extend_desugared(iterator);
        vector
    }
}

impl<T, A: OwnedAllocator> IntoIterator for AllocVec<T, A> {
    type Item = T;
    type IntoIter = AllocIntoIter<T, A>;

    /// Creates a consuming iterator, that is, one that moves each value out of
    /// the vector (from start to end). The vector cannot be used after calling
    /// this.
    ///
    /// # Examples
    ///
    /// ```
    /// let v = vec!["a".to_string(), "b".to_string()];
    /// for s in v.into_iter() {
    ///     // s has type String, not &String
    ///     println!("{}", s);
    /// }
    /// ```
    #[inline]
    fn into_iter(mut self) -> AllocIntoIter<T, A> {
        unsafe {
            let ptr = self.as_mut_ptr();
            intrinsics::assume(!ptr.is_null());
            let begin = ptr as *const T;
            let end = if mem::size_of::<T>() == 0 {
                intrinsics::arith_offset(ptr as *const i8, self.len() as isize) as *const T
            } else {
                ptr.offset(self.len() as isize) as *const T
            };
            let buf = ptr::read(&self.buf);
            mem::forget(self);
            AllocIntoIter {
                _buf: buf,
                ptr: begin,
                end: end,
            }
        }
    }
}

impl<'a, T, A: OwnedAllocator> IntoIterator for &'a AllocVec<T, A> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> slice::Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T, A: OwnedAllocator> IntoIterator for &'a mut AllocVec<T, A> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(mut self) -> slice::IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T, A: OwnedAllocator> Extend<T> for AllocVec<T, A> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend_desugared(iter.into_iter())
    }
}

impl<T, A: OwnedAllocator> AllocVec<T, A> {
    fn extend_desugared<I: Iterator<Item = T>>(&mut self, mut iterator: I) {
        // This function should be the moral equivalent of:
        //
        //      for item in iterator {
        //          self.push(item);
        //      }
        while let Some(element) = iterator.next() {
            let len = self.len();
            if len == self.capacity() {
                let (lower, _) = iterator.size_hint();
                self.reserve(lower.saturating_add(1));
            }
            unsafe {
                ptr::write(self.get_unchecked_mut(len), element);
                // NB can't overflow since we would have had to alloc the address space
                self.set_len(len + 1);
            }
        }
    }
}

impl<'a, T: 'a + Copy, A: OwnedAllocator> Extend<&'a T> for AllocVec<T, A> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().cloned());
    }
}

impl<T: PartialEq, A: OwnedAllocator> PartialEq for AllocVec<T, A> {
    #[inline]
    fn eq(&self, other: &AllocVec<T, A>) -> bool {
        return self[..] == other[..];
    }
}
impl<T: PartialOrd, A: OwnedAllocator> PartialOrd for AllocVec<T, A> {
    #[inline]
    fn partial_cmp(&self, other: &AllocVec<T, A>) -> Option<cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}
impl<T: Eq, A: OwnedAllocator> Eq for AllocVec<T, A> {}

impl<T: Ord, A: OwnedAllocator> Ord for AllocVec<T, A> {
    #[inline]
    fn cmp(&self, other: &AllocVec<T, A>) -> cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T, A: OwnedAllocator> Drop for AllocVec<T, A> {
    fn drop(&mut self) {
        unsafe {
            // use drop for [T]
            ptr::drop_in_place(&mut self[..]);
        }
        // RawVec handles deallocation
    }
}

impl<T, A: OwnedAllocator + Default> Default for AllocVec<T, A> {
    fn default() -> AllocVec<T, A> {
        AllocVec::new()
    }
}

impl<T: fmt::Debug, A: OwnedAllocator> fmt::Debug for AllocVec<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, A: OwnedAllocator> AsRef<AllocVec<T, A>> for AllocVec<T, A> {
    fn as_ref(&self) -> &AllocVec<T, A> {
        self
    }
}

impl<T, A: OwnedAllocator> AsMut<AllocVec<T, A>> for AllocVec<T, A> {
    fn as_mut(&mut self) -> &mut AllocVec<T, A> {
        self
    }
}

impl<T, A: OwnedAllocator> AsRef<[T]> for AllocVec<T, A> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, A: OwnedAllocator> AsMut<[T]> for AllocVec<T, A> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'a, T: Clone, A: OwnedAllocator + Default> From<&'a [T]> for AllocVec<T, A> {
    fn from(s: &'a [T]) -> AllocVec<T, A> {
        let mut ret: AllocVec<T, A> = Default::default();
        ret.extend_from_slice(s);
        return ret;
    }
}

impl<'a, A: OwnedAllocator+Default> From<&'a str> for AllocVec<u8, A> {
    fn from(s: &'a str) -> AllocVec<u8, A> {
        From::from(s.as_bytes())
    }
}


/// /////////////////////////////////////////////////////////////////////////////
/// Iterators
/// /////////////////////////////////////////////////////////////////////////////

/// An iterator that moves out of a vector.
pub struct AllocIntoIter<T, A: OwnedAllocator> {
    _buf: AllocRawVec<T, A>,
    ptr: *const T,
    end: *const T,
}

impl<T, A: OwnedAllocator> Iterator for AllocIntoIter<T, A> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        unsafe {
            if self.ptr == self.end {
                None
            } else {
                if mem::size_of::<T>() == 0 {
                    // purposefully don't use 'ptr.offset' because for
                    // vectors with 0-size elements this would return the
                    // same pointer.
                    self.ptr = intrinsics::arith_offset(self.ptr as *const i8, 1) as *const T;

                    // Use a non-null pointer value
                    Some(ptr::read(heap::EMPTY as *mut T))
                } else {
                    let old = self.ptr;
                    self.ptr = self.ptr.offset(1);

                    Some(ptr::read(old))
                }
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let diff = (self.end as usize) - (self.ptr as usize);
        let size = mem::size_of::<T>();
        let exact = diff / (if size == 0 { 1 } else { size });
        (exact, Some(exact))
    }

    #[inline]
    fn count(self) -> usize {
        self.size_hint().0
    }
}

impl<T, A: OwnedAllocator> DoubleEndedIterator for AllocIntoIter<T, A> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        unsafe {
            if self.end == self.ptr {
                None
            } else {
                if mem::size_of::<T>() == 0 {
                    // See above for why 'ptr.offset' isn't used
                    self.end = intrinsics::arith_offset(self.end as *const i8, -1) as *const T;

                    // Use a non-null pointer value
                    Some(ptr::read(heap::EMPTY as *mut T))
                } else {
                    self.end = self.end.offset(-1);

                    Some(ptr::read(self.end))
                }
            }
        }
    }
}

impl<T, A: OwnedAllocator> ExactSizeIterator for AllocIntoIter<T, A> {}

//impl<T: Clone, A: OwnedAllocator> Clone for AllocIntoIter<T, A> {
//    fn clone(&self) -> AllocIntoIter<T, A> {
//        unsafe { slice::from_raw_parts(self.ptr, self.len()).to_owned().into_iter() }
//    }
//}

impl<T, A: OwnedAllocator> Drop for AllocIntoIter<T, A> {
    fn drop(&mut self) {
        // destroy the remaining elements
        for _x in self {}

        // RawVec handles deallocation
    }
}

/// A draining iterator for `AllocVec<T,A>`.
pub struct AllocDrain<'a, T: 'a, A: OwnedAllocator> {
    /// Index of tail to preserve
    tail_start: usize,
    /// Length of tail
    tail_len: usize,
    /// Current remaining range to remove
    iter: slice::IterMut<'a, T>,
    vec: *mut AllocVec<T, A>,
}

impl<'a, T, A: OwnedAllocator> Iterator for AllocDrain<'a, T, A> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter.next().map(|elt| unsafe { ptr::read(elt as *const _) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T, A: OwnedAllocator> DoubleEndedIterator for AllocDrain<'a, T, A> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back().map(|elt| unsafe { ptr::read(elt as *const _) })
    }
}

impl<'a, T, A: OwnedAllocator> Drop for AllocDrain<'a, T, A> {
    fn drop(&mut self) {
        // exhaust self first
        while let Some(_) = self.next() {}

        if self.tail_len > 0 {
            unsafe {
                let source_vec = &mut *self.vec;
                // memmove back untouched tail, update to new length
                let start = source_vec.len();
                let tail = self.tail_start;
                let src = source_vec.as_ptr().offset(tail as isize);
                let dst = source_vec.as_mut_ptr().offset(start as isize);
                ptr::copy(src, dst, self.tail_len);
                source_vec.set_len(start + self.tail_len);
            }
        }
    }
}


impl<'a, T, A: OwnedAllocator> ExactSizeIterator for AllocDrain<'a, T, A> {}
