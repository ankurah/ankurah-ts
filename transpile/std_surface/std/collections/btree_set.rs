//! `std::collections::btree_set`

pub struct BTreeSet<T>;

impl<T> BTreeSet<T> {
    pub fn new() -> BTreeSet<T> { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn clear(&mut self) { todo!() }
    pub fn iter(&self) -> std::collections::btree_set::Iter<'_, T> { todo!() }
}

impl<T: Ord> BTreeSet<T> {
    pub fn insert(&mut self, value: T) -> bool { todo!() }
    pub fn replace(&mut self, value: T) -> Option<T> { todo!() }
    pub fn append(&mut self, other: &mut BTreeSet<T>) { todo!() }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, f: F) { todo!() }

    pub fn contains<Q: ?Sized + Ord>(&self, value: &Q) -> bool where T: Borrow<Q> { todo!() }
    pub fn get<Q: ?Sized + Ord>(&self, value: &Q) -> Option<&T> where T: Borrow<Q> { todo!() }
    pub fn remove<Q: ?Sized + Ord>(&mut self, value: &Q) -> bool where T: Borrow<Q> { todo!() }
    pub fn take<Q: ?Sized + Ord>(&mut self, value: &Q) -> Option<T> where T: Borrow<Q> { todo!() }

    pub fn first(&self) -> Option<&T> { todo!() }
    pub fn last(&self) -> Option<&T> { todo!() }
    pub fn pop_first(&mut self) -> Option<T> { todo!() }
    pub fn pop_last(&mut self) -> Option<T> { todo!() }
    pub fn range<K: ?Sized + Ord, R: RangeBounds<K>>(&self, range: R) -> Range<'_, T> where T: Borrow<K> { todo!() }

    pub fn difference<'a>(&'a self, other: &'a BTreeSet<T>) -> Difference<'a, T> { todo!() }
    pub fn symmetric_difference<'a>(&'a self, other: &'a BTreeSet<T>) -> SymmetricDifference<'a, T> { todo!() }
    pub fn intersection<'a>(&'a self, other: &'a BTreeSet<T>) -> Intersection<'a, T> { todo!() }
    pub fn union<'a>(&'a self, other: &'a BTreeSet<T>) -> Union<'a, T> { todo!() }
    pub fn is_disjoint(&self, other: &BTreeSet<T>) -> bool { todo!() }
    pub fn is_subset(&self, other: &BTreeSet<T>) -> bool { todo!() }
    pub fn is_superset(&self, other: &BTreeSet<T>) -> bool { todo!() }
}

pub struct Iter<'a, T>;
pub struct IntoIter<T>;
pub struct Range<'a, T>;
pub struct Difference<'a, T>;
pub struct SymmetricDifference<'a, T>;
pub struct Intersection<'a, T>;
pub struct Union<'a, T>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> DoubleEndedIterator for Iter<'a, T> { fn next_back(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> ExactSizeIterator for Iter<'a, T> { fn len(&self) -> usize { todo!() } }
impl<'a, T> Clone for Iter<'a, T> { fn clone(&self) -> Iter<'a, T> { todo!() } }
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<T> DoubleEndedIterator for IntoIter<T> { fn next_back(&mut self) -> Option<T> { todo!() } }
impl<'a, T> Iterator for Range<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T: Ord> Iterator for Difference<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T: Ord> Iterator for SymmetricDifference<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T: Ord> Iterator for Intersection<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T: Ord> Iterator for Union<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }

impl<T> IntoIterator for BTreeSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}
impl<'a, T> IntoIterator for &'a BTreeSet<T> {
    type Item = &'a T;
    type IntoIter = std::collections::btree_set::Iter<'a, T>;
    fn into_iter(self) -> std::collections::btree_set::Iter<'a, T> { todo!() }
}

impl<T: Ord> FromIterator<T> for BTreeSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> BTreeSet<T> { todo!() }
}
impl<T: Ord> Extend<T> for BTreeSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { todo!() }
}

impl<T: Clone> Clone for BTreeSet<T> { fn clone(&self) -> BTreeSet<T> { todo!() } }
impl<T: Debug> Debug for BTreeSet<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Default for BTreeSet<T> { fn default() -> BTreeSet<T> { todo!() } }
impl<T: PartialEq<T>> PartialEq for BTreeSet<T> { fn eq(&self, other: &BTreeSet<T>) -> bool { todo!() } }
impl<T: Eq> Eq for BTreeSet<T> {}
impl<T: Ord> Ord for BTreeSet<T> { fn cmp(&self, other: &BTreeSet<T>) -> std::cmp::Ordering { todo!() } }
