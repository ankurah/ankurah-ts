//! `std::vec`
//!
//! Only Vec's own inherent methods live here. `first`, `last`, `get`, `iter`,
//! `contains`, `sort*`, `binary_search*`, `split_last`, `join`, `to_vec` and the
//! rest are slice methods reached through `Deref<Target = [T]>`, and are
//! declared in `slice.rs` so that the engine walks the same deref chain rustc
//! does.

pub struct Vec<T>;

impl<T> Vec<T> {
    pub fn new() -> Vec<T> { todo!() }
    pub fn with_capacity(capacity: usize) -> Vec<T> { todo!() }

    pub fn capacity(&self) -> usize { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }

    pub fn reserve(&mut self, additional: usize) { todo!() }
    pub fn shrink_to_fit(&mut self) { todo!() }
    pub fn into_boxed_slice(self) -> Box<[T]> { todo!() }
    pub fn as_slice(&self) -> &[T] { todo!() }
    pub fn as_mut_slice(&mut self) -> &mut [T] { todo!() }
    pub fn as_ptr(&self) -> *const T { todo!() }
    pub fn as_mut_ptr(&mut self) -> *mut T { todo!() }

    pub fn push(&mut self, value: T) { todo!() }
    pub fn pop(&mut self) -> Option<T> { todo!() }
    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> { todo!() }
    pub fn insert(&mut self, index: usize, element: T) { todo!() }
    pub fn remove(&mut self, index: usize) -> T { todo!() }
    pub fn swap_remove(&mut self, index: usize) -> T { todo!() }
    pub fn truncate(&mut self, len: usize) { todo!() }
    pub fn clear(&mut self) { todo!() }
    pub fn append(&mut self, other: &mut Vec<T>) { todo!() }
    pub fn split_off(&mut self, at: usize) -> Vec<T> { todo!() }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, f: F) { todo!() }
    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, f: F) { todo!() }
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T> { todo!() }
    pub fn dedup_by_key<F: FnMut(&mut T) -> K, K: PartialEq<K>>(&mut self, key: F) { todo!() }
    pub fn dedup_by<F: FnMut(&mut T, &mut T) -> bool>(&mut self, same_bucket: F) { todo!() }
}

impl<T: Clone> Vec<T> {
    pub fn resize(&mut self, new_len: usize, value: T) { todo!() }
    pub fn extend_from_slice(&mut self, other: &[T]) { todo!() }
    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, src: R) { todo!() }
}

impl<T: PartialEq<T>> Vec<T> {
    pub fn dedup(&mut self) { todo!() }
}

impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { todo!() }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] { todo!() }
}

impl<T: Clone> Clone for Vec<T> { fn clone(&self) -> Vec<T> { todo!() } }
impl<T: Debug> Debug for Vec<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Default for Vec<T> { fn default() -> Vec<T> { todo!() } }
impl<T: PartialEq<T>> PartialEq for Vec<T> { fn eq(&self, other: &Vec<T>) -> bool { todo!() } }
impl<T: Eq> Eq for Vec<T> {}
impl<T: PartialOrd<T>> PartialOrd for Vec<T> { fn partial_cmp(&self, other: &Vec<T>) -> Option<std::cmp::Ordering> { todo!() } }
impl<T: Ord> Ord for Vec<T> { fn cmp(&self, other: &Vec<T>) -> std::cmp::Ordering { todo!() } }
impl<T: PartialEq<T>> PartialEq<[T]> for Vec<T> { fn eq(&self, other: &[T]) -> bool { todo!() } }
impl<T: PartialEq<T>> PartialEq<&[T]> for Vec<T> { fn eq(&self, other: &&[T]) -> bool { todo!() } }

impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Vec<T> { todo!() }
}

impl<T> Extend<T> for Vec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { todo!() }
}

impl<'a, T: 'a + Copy> Extend<&'a T> for Vec<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) { todo!() }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}

impl<'a, T> IntoIterator for &'a Vec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> std::slice::Iter<'a, T> { todo!() }
}

impl<'a, T> IntoIterator for &'a mut Vec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> std::slice::IterMut<'a, T> { todo!() }
}

pub struct IntoIter<T>;
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<T> DoubleEndedIterator for IntoIter<T> { fn next_back(&mut self) -> Option<T> { todo!() } }
impl<T> ExactSizeIterator for IntoIter<T> { fn len(&self) -> usize { todo!() } }

pub struct Drain<'a, T>;
impl<'a, T> Iterator for Drain<'a, T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<'a, T> DoubleEndedIterator for Drain<'a, T> { fn next_back(&mut self) -> Option<T> { todo!() } }
impl<'a, T> ExactSizeIterator for Drain<'a, T> { fn len(&self) -> usize { todo!() } }
