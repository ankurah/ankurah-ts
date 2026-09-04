//! `append-only-vec` 0.1.8
//!
//! Not on the deliverable's list. `core/src/transaction.rs` stages entities in
//! one so that `&Entity` references handed out earlier stay valid as later
//! entities are added — `push` returns the index, not `()`, and that return is
//! the transaction's entity handle. The return type is the whole reason this
//! declaration matters.

pub struct AppendOnlyVec<T>;

impl<T> AppendOnlyVec<T> {
    pub fn new() -> AppendOnlyVec<T> { todo!() }
    /// Returns the index it pushed to, and `core/src/transaction.rs` uses that
    /// index as the entity handle. `&self`, not `&mut self`: that is the point
    /// of the type, and the reason references handed out earlier stay valid.
    pub fn push(&self, val: T) -> usize { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn extend(&self, iter: impl IntoIterator<Item = T>) { todo!() }
    /// 0.1.8 returns an opaque `impl Trait`, not a named iterator, and the
    /// capabilities on it are part of the return type.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator { todo!() }
}

impl<T> std::ops::Index<usize> for AppendOnlyVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T { todo!() }
}

impl<T> IndexMut<usize> for AppendOnlyVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T { todo!() }
}

pub struct IntoIter<T>;

impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }

// Only the owned form implements `IntoIterator`; `for x in &vec` does not
// compile against 0.1.8, and declaring it would make a broken loop resolve.
impl<T> IntoIterator for AppendOnlyVec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}

impl<T> FromIterator<T> for AppendOnlyVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> AppendOnlyVec<T> { todo!() }
}

impl<T> From<Vec<T>> for AppendOnlyVec<T> { fn from(v: Vec<T>) -> AppendOnlyVec<T> { todo!() } }
impl<T: Clone> Clone for AppendOnlyVec<T> { fn clone(&self) -> AppendOnlyVec<T> { todo!() } }
impl<T> Default for AppendOnlyVec<T> { fn default() -> AppendOnlyVec<T> { todo!() } }
impl<T: Debug> Debug for AppendOnlyVec<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
