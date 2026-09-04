//! `std::collections::binary_heap`
//!
//! `storage/common/src/sorting.rs` builds a `BinaryHeap<HeapItem<S::Item>>` for
//! the top-k merge, so this is corpus surface, not a neighbour.

pub struct BinaryHeap<T>;

impl<T> BinaryHeap<T> {
    pub fn new() -> BinaryHeap<T> { todo!() }
    pub fn with_capacity(capacity: usize) -> BinaryHeap<T> { todo!() }
}

impl<T: Ord> BinaryHeap<T> {
    pub fn push(&mut self, item: T) { todo!() }
    pub fn pop(&mut self) -> Option<T> { todo!() }
    pub fn peek(&self) -> Option<&T> { todo!() }
    pub fn into_sorted_vec(self) -> Vec<T> { todo!() }
    pub fn append(&mut self, other: &mut BinaryHeap<T>) { todo!() }
}

impl<T> BinaryHeap<T> {
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn clear(&mut self) { todo!() }
    pub fn iter(&self) -> std::collections::binary_heap::Iter<'_, T> { todo!() }
    pub fn into_vec(self) -> Vec<T> { todo!() }
}

pub struct Iter<'a, T>;
pub struct IntoIter<T>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }

impl<T> IntoIterator for BinaryHeap<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}
impl<'a, T> IntoIterator for &'a BinaryHeap<T> {
    type Item = &'a T;
    type IntoIter = std::collections::binary_heap::Iter<'a, T>;
    fn into_iter(self) -> std::collections::binary_heap::Iter<'a, T> { todo!() }
}
impl<T: Ord> FromIterator<T> for BinaryHeap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> BinaryHeap<T> { todo!() }
}
impl<T: Ord> Extend<T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { todo!() }
}
impl<T: Clone> Clone for BinaryHeap<T> { fn clone(&self) -> BinaryHeap<T> { todo!() } }
impl<T> Default for BinaryHeap<T> { fn default() -> BinaryHeap<T> { todo!() } }
