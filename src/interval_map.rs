use std::marker::PhantomData;
use std::ops::Range;
use std::mem::size_of;
use std::ops::Shl;
use std::ops::Shr;
use std::ops::Sub;
use std::num::One;
use std::num::Zero;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Error;
use std::fmt::Binary;
use std::fmt::UpperHex;
use std::fmt::LowerHex;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::Add;
use std::borrow::BorrowMut;
use std::borrow::Borrow;
use std::hash::Hash;
use std::collections::HashMap;
use rand::thread_rng;
use std::mem::swap;
use rand::Rng;
use std::fmt;
pub trait Bounded {
    fn min_value() -> Self;
    fn max_value() -> Self;
}
impl Bounded for u8 {
    fn min_value() -> Self {
        return u8::min_value();
    }
    fn max_value() -> Self {
        return u8::max_value();
    }
}
impl Bounded for u16 {
    fn min_value() -> Self {
        return u16::min_value();
    }
    fn max_value() -> Self {
        return u16::max_value();
    }
}
impl Bounded for u32 {
    fn min_value() -> Self {
        return u32::min_value();
    }
    fn max_value() -> Self {
        return u32::max_value();
    }
}
impl Bounded for u64 {
    fn min_value() -> Self {
        return u64::min_value();
    }
    fn max_value() -> Self {
        return u64::max_value();
    }
}
impl Bounded for usize {
    fn min_value() -> Self {
        return usize::min_value();
    }
    fn max_value() -> Self {
        return usize::max_value();
    }
}
pub trait FixedUnsigned
    : One + Zero + Shl<usize, Output = Self> + Ord + Copy + BitAnd<Self, Output = Self> +
    Sub<Self,Output=Self> + BitOr<Self,Output=Self> + Shr<usize,Output=Self> + Bounded +
    Binary + LowerHex + UpperHex + PartialOrd<Self> + Add<Self,Output=Self>
    {
    fn overflowing_add(self, other: Self) -> (Self, bool);
    fn overflowing_sub(self, other: Self) -> (Self, bool);
    fn checked_add(self, other: Self) -> Option<Self>;
    fn checked_sub(self, other: Self) -> Option<Self>;
}
impl FixedUnsigned for u8 {
    fn overflowing_add(self, other: Self) -> (Self, bool) {
        return u8::overflowing_add(self, other);
    }
    fn overflowing_sub(self, other: Self) -> (Self, bool) {
        return u8::overflowing_sub(self, other);
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        return u8::checked_add(self, other);
    }
    fn checked_sub(self, other: Self) -> Option<Self> {
        return u8::checked_sub(self, other);
    }
}
impl FixedUnsigned for u16 {
    fn overflowing_add(self, other: Self) -> (Self, bool) {
        return u16::overflowing_add(self, other);
    }
    fn overflowing_sub(self, other: Self) -> (Self, bool) {
        return u16::overflowing_sub(self, other);
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        return u16::checked_add(self, other);
    }
    fn checked_sub(self, other: Self) -> Option<Self> {
        return u16::checked_sub(self, other);
    }
}
impl FixedUnsigned for u32 {
    fn overflowing_add(self, other: Self) -> (Self, bool) {
        return u32::overflowing_add(self, other);
    }
    fn overflowing_sub(self, other: Self) -> (Self, bool) {
        return u32::overflowing_sub(self, other);
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        return u32::checked_add(self, other);
    }
    fn checked_sub(self, other: Self) -> Option<Self> {
        return u32::checked_sub(self, other);
    }
}
impl FixedUnsigned for u64 {
    fn overflowing_add(self, other: Self) -> (Self, bool) {
        return u64::overflowing_add(self, other);
    }
    fn overflowing_sub(self, other: Self) -> (Self, bool) {
        return u64::overflowing_sub(self, other);
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        return u64::checked_add(self, other);
    }
    fn checked_sub(self, other: Self) -> Option<Self> {
        return u64::checked_sub(self, other);
    }
}
impl FixedUnsigned for usize {
    fn overflowing_add(self, other: Self) -> (Self, bool) {
        return usize::overflowing_add(self, other);
    }
    fn overflowing_sub(self, other: Self) -> (Self, bool) {
        return usize::overflowing_sub(self, other);
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        return usize::checked_add(self, other);
    }
    fn checked_sub(self, other: Self) -> Option<Self> {
        return usize::checked_sub(self, other);
    }
}
fn high_bit<T>() -> T
    where T: FixedUnsigned
{
    (T::one() << (8 * size_of::<T>() - 1))
}
fn view_round_down<T>(x: T) -> (bool, T)
    where T: FixedUnsigned
{
    (x & high_bit() != T::zero(), x << 1)
}
fn view_round_up<T>(x: T) -> (bool, T)
    where T: FixedUnsigned
{
    (x & high_bit() != T::zero(), (x << 1) | T::one())
}
fn hide<T>(head: bool, tail: T) -> T
    where T: FixedUnsigned
{
    (if head { high_bit() } else { T::zero() }) | (tail >> 1)
}
fn split<T>(int: Interval<T>) -> (Option<Interval<T>>, Option<Interval<T>>)
    where T: FixedUnsigned
{
    fn yes<T>(front: T, back: T) -> Option<Interval<T>>
        where T: FixedUnsigned
    {
        return Some(Interval {
            front: front,
            back: back,
        });
    }
    let (front_head, front_tail) = view_round_down(int.front);
    let (back_head, back_tail) = view_round_up(int.back);
    if front_head {
        (None, yes(front_tail, back_tail))
    } else if back_head {
        (yes(front_tail, T::max_value()), yes(T::min_value(), back_tail))
    } else {
        (yes(front_tail, back_tail), None)
    }
}

#[derive(Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct Interval<T> {
    front: T,
    back: T,
}
impl<T> Interval<T>
    where T: FixedUnsigned
{
    pub fn subset(self, other: Self) -> bool {
        return self.front >= other.front && self.back <= other.back;
    }
    pub fn len(self) -> Option<T> {
        return (self.back - self.front).checked_add(T::one());
    }
}
#[derive(PartialEq,Eq,Debug)]
pub enum IntervalIter<T> {
    Bounded(T, T),
    Unbounded(Option<T>),
}
impl<T> Interval<T>
    where T: FixedUnsigned
{
    pub fn contains(self, x: T) -> bool {
        return self.front <= x && x <= self.back;
    }
    pub fn iter(self) -> IntervalIter<T> {
        match self.back.checked_add(T::one()) {
            None => IntervalIter::Unbounded(Some(self.front)),
            Some(end) => IntervalIter::Bounded(self.front, end),
        }
    }
}
impl<T> Iterator for IntervalIter<T>
    where T: FixedUnsigned
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match *self {
            IntervalIter::Bounded(ref mut start, ref mut end) => {
                if *start == *end {
                    None
                } else {
                    let ret = *start;
                    *start = *start + T::one();
                    return Some(ret);
                }
            }
            IntervalIter::Unbounded(ref mut head) => {
                let ret = *head;
                match *head {
                    None => {}
                    Some(head_value) => {
                        *head = head_value.checked_add(T::one());
                    }
                }
                return ret;
            }
        }
    }
}
pub fn interval<T>(front: T, back: T) -> Interval<T> {
    return Interval {
        front: front,
        back: back,
    };
}
impl<T> Debug for Interval<T>
    where T: LowerHex + UpperHex
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "[0x{:x}, 0x{:x}]", self.front, self.back)
    }
}
pub trait IntervalMap<'a, K, V>
    where K: FixedUnsigned,
          V: Eq
{
    type Iter: Iterator<Item = (Interval<K>, Option<V>)>;
    fn iter(&'a self, key: K) -> Self::Iter;
    fn get(&'a self, key: K) -> Option<V>;
    fn get_interval(&'a self, key: K) -> (Interval<K>, Option<V>);
    fn fill(&'a mut self, interval: Interval<K>, value: Option<V>);
    fn get_first(&'a self, interval: Interval<K>) -> Option<(Interval<K>, V)> {
        let (front_interval, front_value) = self.get_interval(interval.front);
        match front_value {
            Some(value) => {
                return Some((front_interval, value));
            }
            None => {}
        }
        if front_interval.back < interval.back {
            let (second_interval, second_value) = self.get_interval(front_interval.back + K::one());
            return Some((second_interval, second_value.expect("Interval after None should be Some")));
        }
        return None;
    }
}
pub struct IntervalMapIter<'a, K, V: 'a, Map: 'a>
    where Map: IntervalMap<'a, K, V>,
          K: FixedUnsigned,
          V: Eq
{
    map: &'a Map,
    next: Option<K>,
    phantom: PhantomData<&'a V>,
}
impl<'a, K, V, Map> IntervalMapIter<'a, K, V, Map>
    where Map: IntervalMap<'a, K, V>,
          K: FixedUnsigned,
          V: Eq
{
    pub fn new(map: &'a Map, key: K) -> Self {
        return IntervalMapIter {
            map: map,
            next: Some(key),
            phantom: PhantomData,
        };
    }
}
impl<'a, K, V, Map> Iterator for IntervalMapIter<'a, K, V, Map>
    where Map: IntervalMap<'a, K, V>,
          K: FixedUnsigned,
          V: Eq
{
    type Item = (Interval<K>, Option<V>);
    fn next(&mut self) -> Option<Self::Item> {
        match self.next {
            None => None,
            Some(next) => {
                let ret = self.map.get_interval(next);
                let back = ret.0.back;
                self.next = back.checked_add(K::one());
                return Some(ret);
            }
        }
    }
}
#[derive(Clone)]
pub enum TreeIntervalMap<K, V>
    where K: FixedUnsigned,
          V: Copy + Eq
{
    Node(Box<(TreeIntervalMap<K, V>, TreeIntervalMap<K, V>)>),
    Empty,
    Leaf(V, PhantomData<K>),
}
impl<'a, K, V> TreeIntervalMap<K, V>
    where K: FixedUnsigned,
          V: Copy + Eq
{
    pub fn new() -> Self {
        return TreeIntervalMap::Empty;
    }
    fn fill_option(&'a mut self, interval: Option<Interval<K>>, value: Option<V>) {
        match interval {
            None => {}
            Some(interval) => self.fill(interval, value),
        }
    }
}
impl<'a, K: 'a, V: 'a> IntervalMap<'a, K, V> for TreeIntervalMap<K, V>
    where K: FixedUnsigned,
          V: Copy + Eq
{
    type Iter = IntervalMapIter<'a, K, V, Self>;
    fn iter(&'a self, key: K) -> Self::Iter {
        return IntervalMapIter::new(self, key);
    }
    fn fill(&'a mut self, interval: Interval<K>, value: Option<V>) {
        if interval.front == K::min_value() && interval.back == K::max_value() {
            match value {
                None => *self = TreeIntervalMap::Empty,
                Some(value) => *self = TreeIntervalMap::Leaf(value, PhantomData),
            }
        } else {
            match *self {
                TreeIntervalMap::Node(_) => {}
                TreeIntervalMap::Leaf(y, p) => {
                    *self = TreeIntervalMap::Node(Box::new((TreeIntervalMap::Leaf(y, p), TreeIntervalMap::Leaf(y, p))))
                }
                TreeIntervalMap::Empty => {
                    *self = TreeIntervalMap::Node(Box::new((TreeIntervalMap::Empty, TreeIntervalMap::Empty)));
                }
            }
            match *self {
                TreeIntervalMap::Node(ref mut b) => {
                    let (ref mut left, ref mut right) = *b.borrow_mut();
                    let (leftint, rightint) = split(interval);
                    left.fill_option(leftint, value);
                    right.fill_option(rightint, value);
                }
                TreeIntervalMap::Leaf(_, _) => unreachable!(),
                TreeIntervalMap::Empty => unreachable!(),
            }
        }
    }
    fn get(&'a self, key: K) -> Option<V> {
        match *self {
            TreeIntervalMap::Node(ref b) => {
                let (ref left, ref right) = *&**b;
                let (head, tail) = view_round_down(key);
                if head {
                    right.get(tail)
                } else {
                    left.get(tail)
                }
            }
            TreeIntervalMap::Leaf(ref y, _) => Some(*y),
            TreeIntervalMap::Empty => None,
        }
    }
    fn get_interval(&'a self, key: K) -> (Interval<K>, Option<V>) {
        match *self {
            TreeIntervalMap::Node(ref b) => {
                let (ref left, ref right) = *&**b;
                let (head, tail) = view_round_down(key);
                if head {
                    let (rightint, rightvalue) = right.get_interval(tail);
                    if rightint.front == K::min_value() {
                        let (leftint, leftvalue) = left.get_interval(K::max_value());
                        if leftvalue == rightvalue {
                            return (interval(hide(false, leftint.front), hide(true, rightint.back)), rightvalue);
                        }
                    }
                    return (interval(hide(true, rightint.front), hide(true, rightint.back)), rightvalue);
                } else {
                    let (leftint, leftvalue) = left.get_interval(tail);
                    if leftint.back == K::max_value() {
                        let (rightint, rightvalue) = right.get_interval(K::min_value());
                        if leftvalue == rightvalue {
                            return (interval(hide(false, leftint.front), hide(true, rightint.back)), leftvalue);
                        }
                    }
                    return (interval(hide(false, leftint.front), hide(false, leftint.back)), leftvalue);
                }
            }
            TreeIntervalMap::Leaf(y, _) => (interval(K::min_value(), K::max_value()), Some(y)),
            TreeIntervalMap::Empty => (interval(K::min_value(), K::max_value()), None),
        }
    }
}
impl<'a, K: 'a, V: 'a> Debug for TreeIntervalMap<K, V>
    where K: FixedUnsigned,
          V: Copy + Eq + Debug
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (interval, value) in self.iter(K::min_value()) {
            match value {
                None => {}
                Some(value) => {
                    try!(write!(f, "{:?} = {:?}, ", interval, value));
                }
            }
        }
        return Ok(());
    }
}
struct HashIntervalMap<K: Hash, V: Copy>(HashMap<K, V>);
impl<'a, K: 'a, V: 'a> HashIntervalMap<K, V>
    where K: Hash + FixedUnsigned,
          V: Copy + Eq
{
    fn new() -> Self {
        return HashIntervalMap(HashMap::new());
    }
}
impl<'a, K: 'a, V: 'a> IntervalMap<'a, K, V> for HashIntervalMap<K, V>
    where K: Hash + FixedUnsigned,
          V: Copy + Eq
{
    type Iter = IntervalMapIter<'a, K, V, Self>;
    fn iter(&'a self, key: K) -> Self::Iter {
        return IntervalMapIter::new(self, key);
    }

    fn get(&'a self, key: K) -> Option<V> {
        match self.0.get(&key) {
            None => None,
            Some(x) => Some(*x),
        }
    }
    fn get_interval(&'a self, key: K) -> (Interval<K>, Option<V>) {
        let value = self.get(key);
        let mut front = key;
        loop {
            match front.checked_sub(K::one()) {
                None => break,
                Some(next) => {
                    if self.get(next) != value {
                        break;
                    } else {
                        front = next;
                    }
                }
            }
        }
        let mut back = key;
        loop {
            match back.checked_add(K::one()) {
                None => break,
                Some(next) => {
                    if self.get(next) != value {
                        break;
                    } else {
                        back = next;
                    }
                }
            }
        }
        (interval(front, back), value)
    }
    fn fill(&'a mut self, interval: Interval<K>, value: Option<V>) {
        let mut iter = interval.front;
        loop {
            match value {
                None => {
                    self.0.remove(&iter);
                }
                Some(value) => {
                    self.0.insert(iter, value);
                }
            }
            match iter.checked_add(K::one()) {
                None => break,
                Some(next) => {
                    if next > interval.back {
                        break;
                    } else {
                        iter = next;
                    }
                }
            }
        }
    }
}
macro_rules! same {
    ($format:expr,$a:expr,$b:expr)=>{
        {
            let x=$a;
            let y=$b;
            if x == y {
                x
            } else {
                panic!($format, x, y);
            }
        }
    }
}
struct CompareIterator<I1, I2>
    where I1: Iterator,
          I2: Iterator<Item = I1::Item>,
          I1::Item: Eq + Debug
{
    iter1: I1,
    iter2: I2,
}
impl<I1, I2> Iterator for CompareIterator<I1, I2>
    where I1: Iterator,
          I2: Iterator<Item = I1::Item>,
          I1::Item: Eq + Debug
{
    type Item = I1::Item;
    fn next(&mut self) -> Option<I1::Item> {
        return same!("CompareIterator {:?} != {:?}", self.iter1.next(), self.iter2.next());
    }
}
struct CompareIntervalMap<K, V, M1, M2>
    where M1: for<'a> IntervalMap<'a, K, V>,
          M2: for<'a> IntervalMap<'a, K, V>,
          K: FixedUnsigned,
          V: Eq
{
    first: M1,
    second: M2,
    phantom: PhantomData<(K, V)>,
}
impl<'a, K, V, M1, M2> CompareIntervalMap<K, V, M1, M2>
    where K: 'a + FixedUnsigned,
          V: 'a + Eq + Copy + Debug,
          M1: 'a + for<'b> IntervalMap<'b, K, V>,
          M2: 'a + for<'b> IntervalMap<'b, K, V>
{
    fn new(first: M1, second: M2) -> Self {
        CompareIntervalMap {
            first: first,
            second: second,
            phantom: PhantomData,
        }
    }
}
impl<'a, K, V, M1, M2> IntervalMap<'a, K, V> for CompareIntervalMap<K, V, M1, M2>
    where K: 'a + FixedUnsigned,
          V: 'a + Eq + Copy + Debug,
          M1: 'a + for<'b> IntervalMap<'b, K, V>,
          M2: 'a + for<'b> IntervalMap<'b, K, V>
{
    type Iter = IntervalMapIter<'a, K, V, Self>;
    fn iter(&'a self, key: K) -> Self::Iter {
        return IntervalMapIter::new(self, key);
    }
    fn get(&'a self, key: K) -> Option<V> {
        return same!("CompareIntervalMap {:?} != {:?}", self.first.get(key), self.second.get(key));
    }
    fn get_interval(&'a self, key: K) -> (Interval<K>, Option<V>) {
        return same!("CompareIntervalMap {:?} != {:?}", self.first.get_interval(key), self.second.get_interval(key));
    }
    fn fill(&'a mut self, interval: Interval<K>, value: Option<V>) {
        self.first.fill(interval, value);
        self.second.fill(interval, value);
    }
}
#[test]
fn interval_iter_test() {
    println!("{:?}", everything::<u8>());
    assert_eq!(everything::<u8>().count(), 256);
    assert_eq!(everything::<u8>().next(), Some(0u8));
    assert_eq!(everything::<u8>().last(), Some(255u8));

    assert_eq!(interval(10u8, 20u8).iter().count(), 11);
    assert_eq!(interval(10u8, 20u8).iter().next(), Some(10u8));
    assert_eq!(interval(10u8, 20u8).iter().last(), Some(20u8));
}

#[test]
pub fn test_split() {
    #[derive(Debug,PartialEq,PartialOrd,Ord,Eq)]
    enum Tree {
        Node(Box<Tree>, Box<Tree>),
        Bottom,
        Top,
    }

    fn make_tree<T: FixedUnsigned>(int: Option<Interval<T>>) -> Tree {
        match int {
            None => Tree::Bottom,
            Some(int) => {
                if int.front == T::zero() && int.back == T::max_value() {
                    Tree::Top
                } else {
                    let (left, right) = split(int);
                    Tree::Node(Box::new(make_tree(left)), Box::new(make_tree(right)))
                }
            }
        }
    }
    fn do_test(front: u8, back: u8) -> Tree {
        make_tree(Some(Interval {
            front: front,
            back: back,
        }))
    }
    fn top() -> Tree {
        Tree::Top
    }
    fn bottom() -> Tree {
        Tree::Bottom
    }
    fn node(x: Tree, y: Tree) -> Tree {
        return Tree::Node(Box::new(x), Box::new(y));
    }
    assert_eq!(do_test(0, 255), top());
    assert_eq!(do_test(0, 127), node(top(), bottom()));
    assert_eq!(do_test(128, 255), node(bottom(), top()));
    assert_eq!(do_test(64, 191), node(node(bottom(), top()), node(top(), bottom())));
    assert_eq!(do_test(0, 0),
               node(node(node(node(node(node(node(node(top(), bottom()), bottom()), bottom()), bottom()), bottom()),
                              bottom()),
                         bottom()),
                    bottom()));
    assert_eq!(do_test(255, 255),
               node(bottom(),
                    node(bottom(),
                         node(bottom(),
                              node(bottom(), node(bottom(), node(bottom(), node(bottom(), node(bottom(), top())))))))));
}
fn everything<T>() -> IntervalIter<T>
    where T: FixedUnsigned
{
    return interval(T::min_value(), T::max_value()).iter();
}
#[test]
fn simple_interval_map_test() {
    let mut map: TreeIntervalMap<u8, u8> = TreeIntervalMap::new();
    let int = interval(4, 199);
    map.fill(int, Some(1));
    for x8 in everything() {
        if int.contains(x8) {
            assert!(map.get(x8) == Some(1));
        } else {
            assert!(map.get(x8) == None);
        }
    }
}
#[test]
#[should_panic(CompareIntervalMap)]
fn compare_interval_map_test() {
    let mut one = HashIntervalMap::new();
    one.fill(interval(1, 2), Some(10));
    let mut two = HashIntervalMap::new();
    two.fill(interval(1, 2), Some(11));
    let map: CompareIntervalMap<u8, u8, _, _> = CompareIntervalMap::new(one, two);
    map.get(1);
}
#[test]
fn tree_vs_hash_test() {
    let mut counter = 0u32;
    let mut map: CompareIntervalMap<u8, u32, _, _> = CompareIntervalMap::new(TreeIntervalMap::new(),
                                                                             HashIntervalMap::new());
    for _ in 0u32..10 {
        let mut front = thread_rng().gen::<u8>();
        let mut back = thread_rng().gen::<u8>();
        if back < front {
            swap(&mut front, &mut back);
        }
        let int = interval(front, back);
        let value = counter;
        counter = counter + 1;
        map.fill(int, Some(value));
        for i in int.iter() {
            assert_eq!(map.get(i), Some(value));
            assert_eq!(map.get_interval(i), (int, Some(value)));
        }
        for i in everything() {
            map.get(i);
            map.get_interval(i);
            map.iter(i).count();
        }
    }
}
