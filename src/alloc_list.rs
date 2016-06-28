use allocator::OwnedAllocator;
use std::ptr::Unique;
use std::mem;
use alloc_raw_box::AllocRawBox;
pub struct List<T, A: OwnedAllocator> {
    alloc: A,
    head: Link<T, A>,
}
impl<T, A: OwnedAllocator> List<T, A> {
    fn new(alloc: A) -> Self {
        return List {
            alloc: alloc,
            head: None,
        };
    }
    fn push(&mut self, value: T) {
        unsafe {
            let node = Node {
                next: replace(&mut self.head, None),
                elem: value,
            };
            self.head = Some(AllocRawBox::new(node, &mut self.alloc));
        }
    }
    fn pop(&mut self)->Option<T>{
        match self.head {
            None => None,
            Some(ptr) => {
                ptr.get_mut();
            }
        }
    }
}
type Link<T, A:OwnedAllocator> = Option<AllocRawBox<Node<T, A>, A>>;
struct Node<T, A:OwnedAllocator> {
    next: Link<T, A>,
    elem: T,
}
