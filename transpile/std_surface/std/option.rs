//! `std::option`

pub enum Option<T> {
    None,
    Some(T),
}

impl<T> Option<T> {
    pub fn is_some(&self) -> bool { todo!() }
    pub fn is_none(&self) -> bool { todo!() }
    pub fn is_some_and(self, f: impl FnOnce(T) -> bool) -> bool { todo!() }
    pub fn is_none_or(self, f: impl FnOnce(T) -> bool) -> bool { todo!() }

    pub fn as_ref(&self) -> Option<&T> { todo!() }
    pub fn as_mut(&mut self) -> Option<&mut T> { todo!() }
    pub fn as_deref(&self) -> Option<&<T as Deref>::Target> where T: Deref { todo!() }
    pub fn as_deref_mut(&mut self) -> Option<&mut <T as Deref>::Target> where T: DerefMut { todo!() }

    pub fn expect(self, msg: &str) -> T { todo!() }
    pub fn unwrap(self) -> T { todo!() }
    pub fn unwrap_or(self, default: T) -> T { todo!() }
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, f: F) -> T { todo!() }
    pub fn unwrap_or_default(self) -> T where T: Default { todo!() }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Option<U> { todo!() }
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U { todo!() }
    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U { todo!() }
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Option<T> { todo!() }

    pub fn ok_or<E>(self, err: E) -> Result<T, E> { todo!() }
    pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> { todo!() }

    pub fn and<U>(self, optb: Option<U>) -> Option<U> { todo!() }
    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, f: F) -> Option<U> { todo!() }
    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> { todo!() }
    pub fn or(self, optb: Option<T>) -> Option<T> { todo!() }
    pub fn or_else<F: FnOnce() -> Option<T>>(self, f: F) -> Option<T> { todo!() }
    pub fn xor(self, optb: Option<T>) -> Option<T> { todo!() }
    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> { todo!() }

    pub fn insert(&mut self, value: T) -> &mut T { todo!() }
    pub fn get_or_insert(&mut self, value: T) -> &mut T { todo!() }
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T { todo!() }
    pub fn take(&mut self) -> Option<T> { todo!() }
    pub fn take_if<P: FnOnce(&mut T) -> bool>(&mut self, predicate: P) -> Option<T> { todo!() }
    pub fn replace(&mut self, value: T) -> Option<T> { todo!() }

    pub fn iter(&self) -> std::option::Iter<'_, T> { todo!() }
    pub fn iter_mut(&mut self) -> std::option::IterMut<'_, T> { todo!() }
}

impl<T> Option<&T> {
    pub fn cloned(self) -> Option<T> where T: Clone { todo!() }
    pub fn copied(self) -> Option<T> where T: Copy { todo!() }
}

impl<T> Option<&mut T> {
    pub fn cloned(self) -> Option<T> where T: Clone { todo!() }
    pub fn copied(self) -> Option<T> where T: Copy { todo!() }
}

impl<T> Option<Option<T>> {
    pub fn flatten(self) -> Option<T> { todo!() }
}

impl<T, E> Option<Result<T, E>> {
    pub fn transpose(self) -> Result<Option<T>, E> { todo!() }
}

impl<T: Clone> Clone for Option<T> { fn clone(&self) -> Option<T> { todo!() } }
impl<T: Copy> Copy for Option<T> {}
impl<T: Debug> Debug for Option<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: PartialEq> PartialEq for Option<T> { fn eq(&self, other: &Option<T>) -> bool { todo!() } }
impl<T: Eq> Eq for Option<T> {}
impl<T: PartialOrd> PartialOrd for Option<T> { fn partial_cmp(&self, other: &Option<T>) -> Option<std::cmp::Ordering> { todo!() } }
impl<T: Ord> Ord for Option<T> { fn cmp(&self, other: &Option<T>) -> std::cmp::Ordering { todo!() } }
impl<T> Default for Option<T> { fn default() -> Option<T> { todo!() } }
impl<T> From<T> for Option<T> { fn from(value: T) -> Option<T> { todo!() } }

pub struct Iter<'a, T>;
pub struct IterMut<'a, T>;
pub struct IntoIter<T>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> Iterator for IterMut<'a, T> { type Item = &'a mut T; fn next(&mut self) -> Option<&'a mut T> { todo!() } }
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }

impl<T> IntoIterator for Option<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}
impl<'a, T> IntoIterator for &'a Option<T> {
    type Item = &'a T;
    type IntoIter = std::option::Iter<'a, T>;
    fn into_iter(self) -> std::option::Iter<'a, T> { todo!() }
}
impl<'a, T> IntoIterator for &'a mut Option<T> {
    type Item = &'a mut T;
    type IntoIter = std::option::IterMut<'a, T>;
    fn into_iter(self) -> std::option::IterMut<'a, T> { todo!() }
}

impl<T> Extend<T> for Option<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { todo!() }
}

impl<A, V: FromIterator<A>> FromIterator<Option<A>> for Option<V> {
    fn from_iter<I: IntoIterator<Item = Option<A>>>(iter: I) -> Option<V> { todo!() }
}
