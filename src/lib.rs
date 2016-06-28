#![feature(alloc, heap_api, unique, oom,zero_one,test,
coerce_unsized, unsize, reflect_marker,collections, core_intrinsics,collections_range )]
extern crate alloc;
extern crate core;
extern crate rand;
extern crate test;
extern crate collections;

mod allocator;
mod util;
mod heap_alloc;
mod arena_alloc;
mod interval_map;
mod checked_alloc;
mod alloc_box;
mod alloc_raw_vec;
mod alloc_vec;
mod simple_alloc;
//mod alloc_list;
mod alloc_raw_box;

use std::mem::size_of;
use std::any::Any;
use std::mem::transmute;
use std::ptr::null_mut;
use alloc::heap::usable_size;
use alloc::heap::allocate;
use alloc::heap::reallocate;
use std::ops::Deref;

// fn main() {
//    unsafe {
//        let mut buffer = allocate(8, 1);
//        let mut old_size = 8;
//        for i in 0..40 {
//            let new_size = usable_size(old_size + 1, 1);
//            let old_buffer = buffer;
//            buffer = reallocate(buffer, old_size, new_size, 1);
//            assert!(!buffer.is_null());
//            println!("from {} to {}", old_size, new_size);
//            if old_buffer != buffer {
//                println!("moved");
//            }
//            old_size = new_size;
//        }
//    }
// }
