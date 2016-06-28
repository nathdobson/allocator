#![feature(alloc, heap_api, unique, oom,zero_one,test,
coerce_unsized, unsize,collections, core_intrinsics,collections_range)]
#![cfg_attr(test, feature(reflect_marker))]
extern crate alloc;
extern crate core;
extern crate rand;
extern crate test;
extern crate collections;

pub mod util;
pub mod interval_map;
pub mod allocator;
pub mod heap_alloc;
pub mod arena_alloc;
pub mod checked_alloc;
pub mod alloc_box;
pub mod alloc_raw_vec;
pub mod alloc_vec;
pub mod simple_alloc;
// mod alloc_list;
pub mod alloc_raw_box;
