//! `std::collections::vec_deque`
//!
//! Not reached by the corpus today. It is declared because a queue is the
//! obvious next data structure for the reactor's pending work and the cost of
//! having it is one file.

pub struct VecDeque<T>;

impl<T> VecDeque<T> {
    pub fn new() -> VecDeque<T> { todo!() }
    pub fn with_capacity(capacity: usize) -> VecDeque<T> { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn clear(&mut self) { todo!() }

    pub fn get(&self, index: usize) -> Option<&T> { todo!() }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> { todo!() }
    pub fn front(&self) -> Option<&T> { todo!() }
    pub fn front_mut(&mut self) -> Option<&mut T> { todo!() }
    pub fn back(&self) -> Option<&T> { todo!() }
    pub fn back_mut(&mut self) -> Option<&mut T> { todo!() }

    pub fn push_front(&mut self, value: T) { todo!() }
    pub fn push_back(&mut self, value: T) { todo!() }
    pub fn pop_front(&mut self) -> Option<T> { todo!() }
    pub fn pop_back(&mut self) -> Option<T> { todo!() }
    pub fn insert(&mut self, index: usize, value: T) { todo!() }
    pub fn remove(&mut self, index: usize) -> Option<T> { todo!() }
    pub fn truncate(&mut self, len: usize) { todo!() }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, f: F) { todo!() }
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T> { todo!() }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> { todo!() }
    pub fn iter_mut(&mut self) -> std::collections::vec_deque::IterMut<'_, T> { todo!() }
    pub fn contains(&self, x: &T) -> bool where T: PartialEq<T> { todo!() }
    pub fn make_contiguous(&mut self) -> &mut [T] { todo!() }
}

pub struct Iter<'a, T>;
pub struct IterMut<'a, T>;
pub struct IntoIter<T>;
pub struct Drain<'a, T>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> DoubleEndedIterator for Iter<'a, T> { fn next_back(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> ExactSizeIterator for Iter<'a, T> { fn len(&self) -> usize { todo!() } }
impl<'a, T> Iterator for IterMut<'a, T> { type Item = &'a mut T; fn next(&mut self) -> Option<&'a mut T> { todo!() } }
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<'a, T> Iterator for Drain<'a, T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }

impl<T> IntoIterator for VecDeque<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}
impl<'a, T> IntoIterator for &'a VecDeque<T> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;
    fn into_iter(self) -> std::collections::vec_deque::Iter<'a, T> { todo!() }
}
impl<'a, T> IntoIterator for &'a mut VecDeque<T> {
    type Item = &'a mut T;
    type IntoIter = std::collections::vec_deque::IterMut<'a, T>;
    fn into_iter(self) -> std::collections::vec_deque::IterMut<'a, T> { todo!() }
}

impl<T> FromIterator<T> for VecDeque<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> VecDeque<T> { todo!() }
}
impl<T> Extend<T> for VecDeque<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { todo!() }
}
impl<T: Clone> Clone for VecDeque<T> { fn clone(&self) -> VecDeque<T> { todo!() } }
impl<T: Debug> Debug for VecDeque<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Default for VecDeque<T> { fn default() -> VecDeque<T> { todo!() } }
