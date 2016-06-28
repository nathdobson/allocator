use allocator::OwnedAllocator;
use std::fmt::Debug;
use std::thread;
use interval_map::TreeIntervalMap;
use interval_map::IntervalMap;
use interval_map::Interval;
use std::fmt;
use allocator::ShareableAllocator;
use util::PowerOfTwo;
#[cfg(test)]
use allocator::SharedAlloc;
#[cfg(test)]
use heap_alloc::HeapAlloc;
#[cfg(test)]
use std::ptr::null_mut;
#[cfg(test)]
use simple_alloc::MockAllocResult;
#[cfg(test)]
use simple_alloc::MockAlloc;
#[derive(Clone,Copy,Eq,Ord,PartialEq,PartialOrd)]
struct Allocation {
    ptr: *mut u8,
    requested_size: usize,
    align: PowerOfTwo,
}
impl Debug for Allocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Allocation {{ align : {} }}", self.align)
    }
}
#[derive(Clone,Copy,Eq,Ord,PartialEq,PartialOrd)]
pub struct CheckedAllocOptions {
    pub ignore_leaks: bool,
}
impl Default for CheckedAllocOptions {
    fn default() -> Self {
        return CheckedAllocOptions { ignore_leaks: false };
    }
}
pub struct CheckedAlloc<A: OwnedAllocator> {
    internal: A,
    allocated: TreeIntervalMap<usize, Allocation>,
    options: CheckedAllocOptions,
}
impl<A: OwnedAllocator> CheckedAlloc<A> {
    pub fn new(alloc: A, options: CheckedAllocOptions) -> Self {
        return CheckedAlloc {
            internal: alloc,
            allocated: TreeIntervalMap::new(),
            options: options,
        };
    }
}
impl<A: OwnedAllocator + Default> Default for CheckedAlloc<A> {
    fn default() -> Self {
        return Self::new(Default::default(), Default::default());
    }
}
impl<'a, A: OwnedAllocator> CheckedAlloc<A> {
    fn handle_allocate(&mut self, ptr: *mut u8, requested_size: usize, align: PowerOfTwo) {
        assert!(align.is_aligned_ptr_mut(ptr),
                "CheckedAlloc: allocated pointer {:X} not aligned to {}",
                ptr as usize,
                align);
        let real_size = unsafe { self.internal.usable_size(requested_size, align) };
        let int = Interval::new(ptr as usize, ptr as usize + real_size - 1);
        match self.allocated.get_first(int) {
            None => {}
            Some((other_int, _)) => {
                panic!("CheckedAlloc: Allocated interval {:?} while {:?} still live.", int, other_int);
            }
        }
        self.allocated.fill(int,
                            Some(Allocation {
                                ptr: ptr,
                                requested_size: requested_size,
                                align: align,
                            }));
    }
    unsafe fn handle_deallocate(&mut self, ptr: *mut u8, size: usize, align: PowerOfTwo) {
        assert!(align.is_aligned_ptr_mut(ptr), "CheckedAlloc: deallocated unaligned pointer");
        println!("{:?}", self.allocated);
        match self.allocated.get_interval(ptr as usize) {
            (_, None) => {
                panic!("CheckedAlloc: Deallocated interval {:?} is not live.",
                       Interval::new(ptr as usize, ptr as usize + size - 1));
            }
            (int, Some(allocation)) => {
                assert!(align == allocation.align, "CheckedAlloc: different alignment");
                assert!(allocation.requested_size <= size, "CheckedAlloc: different size ");
                assert!(size <= self.internal.usable_size(allocation.requested_size, align),
                        "CheckedAlloc different size ");
                self.allocated.fill(int, None);
            }
        }
    }
}
unsafe impl<A: OwnedAllocator> OwnedAllocator for CheckedAlloc<A> {
    unsafe fn allocate(&mut self, size: usize, align: PowerOfTwo) -> *mut u8 {
        assert!(size > 0, "CheckedAlloc");
        let ret = self.internal.allocate(size, align);
        if ret.is_null() {
            return ret;
        }
        self.handle_allocate(ret, size, align);
        return ret;
    }
    unsafe fn reallocate(&mut self, ptr: *mut u8, old_size: usize, size: usize, align: PowerOfTwo) -> *mut u8 {
        assert!(size > 0, "CheckedAlloc");
        self.handle_deallocate(ptr, old_size, align);
        let ret = self.internal.reallocate(ptr, old_size, size, align);
        if ret.is_null() {
            self.handle_allocate(ret, old_size, align);
        } else {
            self.handle_allocate(ret, size, align);
        }
        return ret;
    }
    unsafe fn reallocate_inplace(&mut self, ptr: *mut u8, old_size: usize, size: usize, align: PowerOfTwo) -> usize {
        assert!(size > 0, "CheckedAlloc");
        self.handle_deallocate(ptr, old_size, align);
        let actual_size = self.internal.reallocate_inplace(ptr, old_size, size, align);
        assert!(actual_size == size || actual_size == old_size, "CheckedAlloc");
        self.handle_allocate(ptr, actual_size, align);
        return actual_size;
    }
    unsafe fn deallocate(&mut self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) {
        assert!(old_size > 0, "CheckedAlloc");
        self.handle_deallocate(ptr, old_size, align);
        self.internal.deallocate(ptr, old_size, align);
    }
    unsafe fn extendable_size(&self, ptr: *mut u8, old_size: usize, align: PowerOfTwo) -> usize {
        assert!(old_size > 0, "CheckedAlloc");
        let ret = self.internal.extendable_size(ptr, old_size, align);
        assert!(ret >= old_size);
        return ret;
    }
    unsafe fn usable_size(&self, size: usize, align: PowerOfTwo) -> usize {
        assert!(size > 0, "CheckedAlloc");
        let ret = self.internal.usable_size(size, align);
        assert!(ret >= size, "CheckedAlloc");
        assert!(align.is_aligned_size(ret), "CheckedAlloc");
        return ret;
    }
}

impl<A: OwnedAllocator> Drop for CheckedAlloc<A> {
    fn drop(&mut self) {
        if !self.options.ignore_leaks && !thread::panicking() {
            let mut count = 0;
            let mut bytes = 0;
            for (int, allocation) in self.allocated.iter(0) {
                match allocation {
                    Some(_) => {
                        count += 1;
                        bytes += int.len().unwrap();
                    }
                    None => {}
                }
            }
            if count > 0 {
                panic!("LeakChecker: {} bytes in {} blocks leaked.", bytes, count);
            }
        }
    }
}
unsafe impl<A> ShareableAllocator for CheckedAlloc<A> where A: ShareableAllocator {}

macro_rules! alloc_panic_tests {
	{$common:ident $(test $name:ident $code:block)* } => {
	    $(
            #[test]
            #[should_panic(expected = "CheckedAlloc")]
            fn $name() {
                unsafe {
                    let $common : SharedAlloc<CheckedAlloc<HeapAlloc>> = Default::default();
                    $code
                }
            }
	    )*
	};
}
#[cfg(test)]
fn align(x: usize) -> PowerOfTwo {
    return PowerOfTwo::new(x);
}
alloc_panic_tests!{
    alloc
    test allocate_bad_size { (&alloc).allocate(0,align(1)); }
    test deallocate_bad_ptr { (&alloc).deallocate(null_mut(),1,align(1)); }
    test deallocate_wrong_size { (&alloc).deallocate((&alloc).allocate(1,align(1)),100,align(1)); }
    test deallocate_wrong_align { (&alloc).deallocate((&alloc).allocate(2,align(2)),2,align(1)); }
    test reallocate_bad_ptr { (&alloc).reallocate(null_mut(),1,2,align(1)); }
    test reallocate_wrong_size { (&alloc).reallocate((&alloc).allocate(1,align(1)),100,1000,align(1)); }
    test reallocate_wrong_align { (&alloc).reallocate((&alloc).allocate(1,align(1)),1,200,align(2)); }
    test reallocate_bad_size { (&alloc).reallocate((&alloc).allocate(1,align(1)),1,0,align(1)); }
}
#[test]
fn allocate_test() {
    unsafe {
        let alloc: SharedAlloc<CheckedAlloc<HeapAlloc>> = Default::default();
        (&alloc).deallocate((&alloc).allocate(1, align(1)), 1, align(1));
        (&alloc).deallocate((&alloc).reallocate((&alloc).allocate(1, align(1)), 1, 100, align(1)), 100, align(1));
        (&alloc).deallocate((&alloc).reallocate((&alloc).allocate(100, align(1)), 100, 1, align(1)), 1, align(1));
    }
}
#[test]
fn test_good_backend() {
    unsafe {
        let alloc = SharedAlloc::new(CheckedAlloc::new(MockAlloc::new(vec![MockAllocResult::Allocate(null_mut()),
                                                              MockAllocResult::Allocate(1 as *mut u8),
                                                              MockAllocResult::Reallocate(null_mut()),
                                                              MockAllocResult::Allocate(2 as *mut u8),
                                                              MockAllocResult::Deallocate,
                                                              MockAllocResult::Deallocate,
                                                              ]),
                                                       Default::default()));
        (&alloc).allocate(1, align(1));
        let b1 = (&alloc).reallocate((&alloc).allocate(1, align(1)), 1, 10, align(1));
        let b2 = (&alloc).allocate(1, align(1));
        (&alloc).deallocate(b1, 1, align(1));
        (&alloc).deallocate(b2, 1, align(1));
    }
}
#[test]
#[should_panic(expected = "CheckedAlloc")]
fn test_overlap() {
    unsafe {
        let alloc = SharedAlloc::new(CheckedAlloc::new(MockAlloc::new(vec![MockAllocResult::Allocate(1 as *mut u8),
                                                                           MockAllocResult::Allocate(2 as *mut u8)]),
                                                       Default::default()));
        (&alloc).allocate(10, align(1));
        (&alloc).allocate(10, align(1));
    }
}
#[test]
#[should_panic(expected = "LeakChecker")]
fn test_allocate_leak() {
    unsafe {
        let alloc: SharedAlloc<CheckedAlloc<HeapAlloc>> = Default::default();
        (&alloc).allocate(1, align(1));
    }
}
