use std::mem;
use std::ptr;
use std::ptr::Unique;
use std::intrinsics;
use std::marker;
use std::ops;
use alloc::oom;
use alloc::heap;
use util::PowerOfTwo;
use allocator::OwnedAllocator;
#[must_use]
pub struct AllocRawBox<T: ?Sized, A: OwnedAllocator> {
    ptr: Unique<T>,
    phantom: marker::PhantomData<*mut A>,
}

impl<T, A: OwnedAllocator> AllocRawBox<T, A> {
    pub fn new(value: T, alloc: &mut A) -> Self {
        unsafe {
            let ptr = if mem::size_of::<T>() == 0 {
                heap::EMPTY as *mut T
            } else {
                let pointer = alloc.allocate(mem::size_of::<T>(), PowerOfTwo::align_of::<T>());
                if pointer.is_null() {
                    oom();
                }
                ptr::write(pointer as *mut T, value);
                pointer as *mut T
            };
            return AllocRawBox {
                ptr: Unique::new(ptr),
                phantom: marker::PhantomData,
            };
        }
    }
    pub unsafe fn into_inner(self, alloc: &mut A) -> T {
        let size = mem::size_of_val::<T>(&**self.ptr);
        let align = mem::align_of_val::<T>(&**self.ptr);
        let result = ptr::read(*self.ptr);
        if size != 0 {
            alloc.deallocate(*self.ptr as *mut u8, size, PowerOfTwo::new(align));
        }
        mem::forget(self);
        return result;
    }
}
impl<T: ?Sized, A: OwnedAllocator> AllocRawBox<T, A> {
    pub unsafe fn delete(self, alloc: &mut A) {
        let size = mem::size_of_val::<T>(&**self.ptr);
        let align = mem::align_of_val::<T>(&**self.ptr);
        intrinsics::drop_in_place::<T>(*self.ptr);
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
            phantom: marker::PhantomData,
        };
    }
    pub fn get_mut(&mut self) -> *mut T {
        return unsafe { self.ptr.get_mut() };
    }
    pub fn get(&self) -> *const T {
        return unsafe { self.ptr.get() };
    }
}

impl<T: ?Sized, U: ?Sized, A> ops::CoerceUnsized<AllocRawBox<U, A>> for AllocRawBox<T, A>
    where T: marker::Unsize<U>,
          A: OwnedAllocator
{
}
