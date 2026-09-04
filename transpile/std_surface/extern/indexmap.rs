//! `indexmap` 2.12.1
//!
//! Not on the deliverable's list. `storage/common/src/planner.rs` relies on
//! insertion order when it groups inequality conjuncts, and
//! `core/src/reactor/subscription_state.rs` builds the update-item map with it.
//! Insertion order is a semantic requirement of the planner, not an
//! optimisation — the plan the sqlite and IndexedDB backends generate depends
//! on it.

pub struct IndexMap<K, V, S = RandomState>;

impl<K, V> IndexMap<K, V, RandomState> {
    pub fn new() -> IndexMap<K, V, RandomState> { todo!() }
    pub fn with_capacity(n: usize) -> IndexMap<K, V, RandomState> { todo!() }
}

impl<K, V, S> IndexMap<K, V, S> {
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn clear(&mut self) { todo!() }
    pub fn iter(&self) -> indexmap::Iter<'_, K, V> { todo!() }
    pub fn iter_mut(&mut self) -> indexmap::IterMut<'_, K, V> { todo!() }
    pub fn keys(&self) -> Keys<'_, K, V> { todo!() }
    pub fn values(&self) -> Values<'_, K, V> { todo!() }
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> { todo!() }
    pub fn into_values(self) -> IntoValues<K, V> { todo!() }
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> { todo!() }
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, keep: F) { todo!() }
}

impl<K: Hash + Eq, V, S: BuildHasher> IndexMap<K, V, S> {
    pub fn insert(&mut self, key: K, value: V) -> Option<V> { todo!() }
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> { todo!() }
    pub fn get<Q: ?Sized + Hash + Eq>(&self, key: &Q) -> Option<&V> where K: Borrow<Q> { todo!() }
    pub fn get_mut<Q: ?Sized + Hash + Eq>(&mut self, key: &Q) -> Option<&mut V> where K: Borrow<Q> { todo!() }
    pub fn contains_key<Q: ?Sized + Hash + Eq>(&self, key: &Q) -> bool where K: Borrow<Q> { todo!() }
    pub fn shift_remove<Q: ?Sized + Hash + Eq>(&mut self, key: &Q) -> Option<V> where K: Borrow<Q> { todo!() }
    pub fn swap_remove<Q: ?Sized + Hash + Eq>(&mut self, key: &Q) -> Option<V> where K: Borrow<Q> { todo!() }
}

pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V> {
    pub fn or_insert(self, default: V) -> &'a mut V { todo!() }
    pub fn or_insert_with<F: FnOnce() -> V>(self, call: F) -> &'a mut V { todo!() }
    pub fn or_default(self) -> &'a mut V where V: Default { todo!() }
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Entry<'a, K, V> { todo!() }
    pub fn key(&self) -> &K { todo!() }
}

pub struct OccupiedEntry<'a, K, V>;
pub struct VacantEntry<'a, K, V>;
pub struct Iter<'a, K, V>;
pub struct IterMut<'a, K, V>;
pub struct IntoIter<K, V>;
pub struct Keys<'a, K, V>;
pub struct Values<'a, K, V>;
pub struct ValuesMut<'a, K, V>;
pub struct IntoValues<K, V>;

impl<'a, K, V> Iterator for Iter<'a, K, V> { type Item = (&'a K, &'a V); fn next(&mut self) -> Option<(&'a K, &'a V)> { todo!() } }
impl<'a, K, V> Iterator for IterMut<'a, K, V> { type Item = (&'a K, &'a mut V); fn next(&mut self) -> Option<(&'a K, &'a mut V)> { todo!() } }
impl<K, V> Iterator for IntoIter<K, V> { type Item = (K, V); fn next(&mut self) -> Option<(K, V)> { todo!() } }
impl<'a, K, V> Iterator for Keys<'a, K, V> { type Item = &'a K; fn next(&mut self) -> Option<&'a K> { todo!() } }
impl<'a, K, V> Iterator for Values<'a, K, V> { type Item = &'a V; fn next(&mut self) -> Option<&'a V> { todo!() } }
impl<'a, K, V> Iterator for ValuesMut<'a, K, V> { type Item = &'a mut V; fn next(&mut self) -> Option<&'a mut V> { todo!() } }
impl<K, V> Iterator for IntoValues<K, V> { type Item = V; fn next(&mut self) -> Option<V> { todo!() } }

impl<K, V, S> IntoIterator for IndexMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> { todo!() }
}
impl<'a, K, V, S> IntoIterator for &'a IndexMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = indexmap::Iter<'a, K, V>;
    fn into_iter(self) -> indexmap::Iter<'a, K, V> { todo!() }
}
impl<'a, K, V, S> IntoIterator for &'a mut IndexMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = indexmap::IterMut<'a, K, V>;
    fn into_iter(self) -> indexmap::IterMut<'a, K, V> { todo!() }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> FromIterator<(K, V)> for IndexMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> IndexMap<K, V, S> { todo!() }
}
impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for IndexMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) { todo!() }
}
impl<K: Clone, V: Clone, S: Clone> Clone for IndexMap<K, V, S> { fn clone(&self) -> IndexMap<K, V, S> { todo!() } }
impl<K: Debug, V: Debug, S> Debug for IndexMap<K, V, S> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<K, V, S: Default> Default for IndexMap<K, V, S> { fn default() -> IndexMap<K, V, S> { todo!() } }

pub struct IndexSet<T, S = RandomState>;

impl<T> IndexSet<T, RandomState> {
    pub fn new() -> IndexSet<T, RandomState> { todo!() }
}

impl<T, S> IndexSet<T, S> {
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn iter(&self) -> SetIter<'_, T> { todo!() }
}

impl<T: Hash + Eq, S: BuildHasher> IndexSet<T, S> {
    pub fn insert(&mut self, value: T) -> bool { todo!() }
    pub fn contains<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> bool where T: Borrow<Q> { todo!() }
    pub fn shift_remove<Q: ?Sized + Hash + Eq>(&mut self, value: &Q) -> bool where T: Borrow<Q> { todo!() }
}

pub struct SetIter<'a, T>;

impl<'a, T> Iterator for SetIter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
