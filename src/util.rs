use std::mem::transmute;
use std::mem::size_of;
use std::mem::align_of;
use alloc::heap::allocate;
use alloc::heap::deallocate;
use std::cmp::max;
use std::ptr::write;
use std::marker::PhantomData;
use std::ptr::drop_in_place;
use std::ptr::null;
use std::ptr::null_mut;
use std::fmt;
use core::marker::{self, Unsize};
use core::ops::{CoerceUnsized, Deref, DerefMut};
use test::Bencher;
use test::black_box;

#[derive(Clone,Copy,Eq,PartialEq,Ord,PartialOrd,Debug)]
pub struct PowerOfTwo(usize);
impl PowerOfTwo {
    pub fn new(x: usize) -> Self {
        assert!(x.is_power_of_two());
        return PowerOfTwo(x);
    }
    pub fn align_size(self, x: usize) -> usize {
        return (x + self.0 - 1) & (usize::max_value() - self.0 + 1);
    }
    pub fn align_ptr_mut<T>(self, x: *mut T) -> *mut T {
        return self.align_size(x as usize) as *mut T;
    }
    pub fn align_ptr_const<T>(self, x: *const T) -> *const T {
        return self.align_size(x as usize) as *const T;
    }
    pub fn is_aligned_ptr_mut<T>(self, x: *mut T) -> bool {
        return (x as usize) % self.0 == 0;
    }
    pub fn is_aligned_size(self, x: usize) -> bool {
        return x % self.0 == 0;
    }
    pub fn align_of<T>() -> PowerOfTwo {
        return Self::new(align_of::<T>());
    }
    pub fn into(self) -> usize {
        return self.0;
    }
}
impl fmt::Display for PowerOfTwo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
// impl From<PowerOfTwo> for usize {
//    fn from(x: PowerOfTwo) -> usize {
//        return x.0;
//    }
// }
#[test]
fn align_test() {
    assert_eq!(0, PowerOfTwo::new(1).align_size(0));
    assert_eq!(1, PowerOfTwo::new(1).align_size(1));
    assert_eq!(2, PowerOfTwo::new(1).align_size(2));
    assert_eq!(3, PowerOfTwo::new(1).align_size(3));

    assert_eq!(0, PowerOfTwo::new(2).align_size(0));
    assert_eq!(2, PowerOfTwo::new(2).align_size(1));
    assert_eq!(2, PowerOfTwo::new(2).align_size(2));
    assert_eq!(4, PowerOfTwo::new(2).align_size(3));

    assert_eq!(0, PowerOfTwo::new(4).align_size(0));
    assert_eq!(4, PowerOfTwo::new(4).align_size(1));
    assert_eq!(4, PowerOfTwo::new(4).align_size(2));
    assert_eq!(4, PowerOfTwo::new(4).align_size(3));
    assert_eq!(4, PowerOfTwo::new(4).align_size(4));
    assert_eq!(8, PowerOfTwo::new(4).align_size(5));
}
pub fn distance<T>(x: *const T, y: *const T) -> usize {
    return ((y as usize) - (x as usize)) / size_of::<T>();
}
use std::mem::forget;
pub struct CheckDrop {
    built: bool,
    dropped: bool,
}
pub struct MustDrop<'a>(&'a mut CheckDrop);
impl Drop for CheckDrop {
    fn drop(&mut self) {
        if self.built {
            assert!(self.dropped)
        }
    }
}
impl CheckDrop {
    pub fn new() -> CheckDrop {
        return CheckDrop {
            built: false,
            dropped: false,
        };
    }
    pub fn build<'a>(&'a mut self) -> MustDrop<'a> {
        self.built = true;
        return MustDrop(self);
    }
}
impl<'a> Drop for MustDrop<'a> {
    fn drop(&mut self) {
        self.0.dropped = true;
    }
}
#[test]
fn must_drop_test() {
    CheckDrop::new();
    let mut temp = CheckDrop::new();
    temp.build();
}
#[test]
#[should_panic]
fn must_drop_panic_test() {
    let mut tmp = CheckDrop::new();
    forget(tmp.build());
}
#[derive(Debug)]
struct Foo<T: ?Sized>(usize, Box<T>);
impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Foo<U>> for Foo<T> {}
#[test]
fn coerce_test() {
    let foo1: Foo<[i32; 4]> = Foo(0xDEADBEEFDEADBEEF, Box::new([1, 2, 3, 4]));
    let foo2: Foo<[i32]> = foo1;
    println!("{:?}", foo2);
}
fn unbox<T>(b: Box<T>) -> T {
    return *b;
}
#[inline(never)]
pub fn do_nothing1(a: usize) -> usize {
    return a * 1 / 1;
}
#[inline(never)]
pub fn do_nothing2(a: usize) -> usize {
    return a * 2 / 2;
}
#[inline(never)]
pub fn do_nothing3(a: usize) -> usize {
    return a * 6 / 6;
}
#[bench]
pub fn bench_do_nothing(b: &mut Bencher) {
    println!("{:x} {:x} {:x}",
             do_nothing1 as *mut u8 as usize,
             do_nothing2 as *mut u8 as usize,
             do_nothing3 as *mut u8 as usize);
}
