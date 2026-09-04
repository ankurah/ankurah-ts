//! `std::collections::hash_set`

pub struct HashSet<T, S = RandomState>;

impl<T> HashSet<T, RandomState> {
    pub fn new() -> HashSet<T, RandomState> { todo!() }
    pub fn with_capacity(capacity: usize) -> HashSet<T, RandomState> { todo!() }
}

impl<T, S> HashSet<T, S> {
    pub fn with_hasher(hasher: S) -> HashSet<T, S> { todo!() }
    pub fn capacity(&self) -> usize { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn iter(&self) -> std::collections::hash_set::Iter<'_, T> { todo!() }
    pub fn drain(&mut self) -> Drain<'_, T> { todo!() }
    pub fn clear(&mut self) { todo!() }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, f: F) { todo!() }
}

impl<T: Eq + Hash, S: BuildHasher> HashSet<T, S> {
    pub fn reserve(&mut self, additional: usize) { todo!() }
    pub fn insert(&mut self, value: T) -> bool { todo!() }
    pub fn replace(&mut self, value: T) -> Option<T> { todo!() }

    pub fn contains<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> bool where T: Borrow<Q> { todo!() }
    pub fn get<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> Option<&T> where T: Borrow<Q> { todo!() }
    pub fn remove<Q: ?Sized + Hash + Eq>(&mut self, value: &Q) -> bool where T: Borrow<Q> { todo!() }
    pub fn take<Q: ?Sized + Hash + Eq>(&mut self, value: &Q) -> Option<T> where T: Borrow<Q> { todo!() }

    pub fn difference<'a>(&'a self, other: &'a HashSet<T, S>) -> Difference<'a, T, S> { todo!() }
    pub fn symmetric_difference<'a>(&'a self, other: &'a HashSet<T, S>) -> SymmetricDifference<'a, T, S> { todo!() }
    pub fn intersection<'a>(&'a self, other: &'a HashSet<T, S>) -> Intersection<'a, T, S> { todo!() }
    pub fn union<'a>(&'a self, other: &'a HashSet<T, S>) -> Union<'a, T, S> { todo!() }
    pub fn is_disjoint(&self, other: &HashSet<T, S>) -> bool { todo!() }
    pub fn is_subset(&self, other: &HashSet<T, S>) -> bool { todo!() }
    pub fn is_superset(&self, other: &HashSet<T, S>) -> bool { todo!() }
}

pub struct Iter<'a, T>;
pub struct IntoIter<T>;
pub struct Drain<'a, T>;
pub struct Difference<'a, T, S>;
pub struct SymmetricDifference<'a, T, S>;
pub struct Intersection<'a, T, S>;
pub struct Union<'a, T, S>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> ExactSizeIterator for Iter<'a, T> { fn len(&self) -> usize { todo!() } }
impl<'a, T> Clone for Iter<'a, T> { fn clone(&self) -> Iter<'a, T> { todo!() } }
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<'a, T> Iterator for Drain<'a, T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Difference<'a, T, S> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for SymmetricDifference<'a, T, S> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Intersection<'a, T, S> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Union<'a, T, S> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }

impl<T, S> IntoIterator for HashSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}
impl<'a, T, S> IntoIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type IntoIter = std::collections::hash_set::Iter<'a, T>;
    fn into_iter(self) -> std::collections::hash_set::Iter<'a, T> { todo!() }
}

impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for HashSet<T, S> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> HashSet<T, S> { todo!() }
}
impl<T: Eq + Hash, S: BuildHasher> Extend<T> for HashSet<T, S> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { todo!() }
}

impl<T: Clone, S: Clone> Clone for HashSet<T, S> { fn clone(&self) -> HashSet<T, S> { todo!() } }
impl<T: Debug, S> Debug for HashSet<T, S> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T, S: Default> Default for HashSet<T, S> { fn default() -> HashSet<T, S> { todo!() } }
impl<T: Eq + Hash, S: BuildHasher> PartialEq for HashSet<T, S> { fn eq(&self, other: &HashSet<T, S>) -> bool { todo!() } }
impl<T: Eq + Hash, S: BuildHasher> Eq for HashSet<T, S> {}
