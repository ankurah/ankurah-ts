//! `std::collections::hash_map`
//!
//! The lookup methods keep Rust's real `Q: ?Sized` / `K: Borrow<Q>` shape.
//! Dropping it would make `map.get("key")` on a `HashMap<String, _>`
//! unresolvable and the failure would look like a corpus problem rather than a
//! stub problem. `std/borrow.rs` carries the impls that discharge the bound.
//!
//! The hasher parameter `S` is kept, with its `RandomState` default, because
//! `core/src/util/iterable.rs` writes `impl<T, S: BuildHasher> Iterable<T> for
//! HashSet<T, S>` — a corpus impl that will not unify against a two-parameter
//! `HashMap`. The allocator parameter is dropped; nothing in ankurah names one.

pub struct HashMap<K, V, S = RandomState>;

impl<K, V> HashMap<K, V, RandomState> {
    pub fn new() -> HashMap<K, V, RandomState> { todo!() }
    pub fn with_capacity(capacity: usize) -> HashMap<K, V, RandomState> { todo!() }
}

impl<K, V, S> HashMap<K, V, S> {
    pub fn with_hasher(hash_builder: S) -> HashMap<K, V, S> { todo!() }
    pub fn capacity(&self) -> usize { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn hasher(&self) -> &S { todo!() }

    pub fn keys(&self) -> Keys<'_, K, V> { todo!() }
    pub fn values(&self) -> Values<'_, K, V> { todo!() }
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> { todo!() }
    pub fn into_keys(self) -> IntoKeys<K, V> { todo!() }
    pub fn into_values(self) -> IntoValues<K, V> { todo!() }
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> { todo!() }
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, K, V> { todo!() }
    pub fn drain(&mut self) -> Drain<'_, K, V> { todo!() }
    pub fn clear(&mut self) { todo!() }
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, f: F) { todo!() }
}

impl<K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S> {
    pub fn reserve(&mut self, additional: usize) { todo!() }
    pub fn shrink_to_fit(&mut self) { todo!() }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> { todo!() }
    pub fn insert(&mut self, k: K, v: V) -> Option<V> { todo!() }

    pub fn get<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> Option<&V> where K: Borrow<Q> { todo!() }
    pub fn get_mut<Q: ?Sized + Hash + Eq>(&mut self, k: &Q) -> Option<&mut V> where K: Borrow<Q> { todo!() }
    pub fn get_key_value<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> Option<(&K, &V)> where K: Borrow<Q> { todo!() }
    pub fn contains_key<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> bool where K: Borrow<Q> { todo!() }
    pub fn remove<Q: ?Sized + Hash + Eq>(&mut self, k: &Q) -> Option<V> where K: Borrow<Q> { todo!() }
    pub fn remove_entry<Q: ?Sized + Hash + Eq>(&mut self, k: &Q) -> Option<(K, V)> where K: Borrow<Q> { todo!() }
}

pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V> {
    pub fn or_insert(self, default: V) -> &'a mut V { todo!() }
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V { todo!() }
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V { todo!() }
    pub fn or_default(self) -> &'a mut V where V: Default { todo!() }
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Entry<'a, K, V> { todo!() }
    pub fn key(&self) -> &K { todo!() }
}

pub struct OccupiedEntry<'a, K, V>;

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K { todo!() }
    pub fn get(&self) -> &V { todo!() }
    pub fn get_mut(&mut self) -> &mut V { todo!() }
    pub fn into_mut(self) -> &'a mut V { todo!() }
    pub fn insert(&mut self, value: V) -> V { todo!() }
    pub fn remove(self) -> V { todo!() }
    pub fn remove_entry(self) -> (K, V) { todo!() }
}

pub struct VacantEntry<'a, K, V>;

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K { todo!() }
    pub fn into_key(self) -> K { todo!() }
    pub fn insert(self, value: V) -> &'a mut V { todo!() }
}

pub struct Iter<'a, K, V>;
pub struct IterMut<'a, K, V>;
pub struct IntoIter<K, V>;
pub struct Keys<'a, K, V>;
pub struct Values<'a, K, V>;
pub struct ValuesMut<'a, K, V>;
pub struct IntoKeys<K, V>;
pub struct IntoValues<K, V>;
pub struct Drain<'a, K, V>;

impl<'a, K, V> Iterator for Iter<'a, K, V> { type Item = (&'a K, &'a V); fn next(&mut self) -> Option<(&'a K, &'a V)> { todo!() } }
impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> { fn len(&self) -> usize { todo!() } }
impl<'a, K, V> Clone for Iter<'a, K, V> { fn clone(&self) -> Iter<'a, K, V> { todo!() } }
impl<'a, K, V> Iterator for IterMut<'a, K, V> { type Item = (&'a K, &'a mut V); fn next(&mut self) -> Option<(&'a K, &'a mut V)> { todo!() } }
impl<K, V> Iterator for IntoIter<K, V> { type Item = (K, V); fn next(&mut self) -> Option<(K, V)> { todo!() } }
impl<'a, K, V> Iterator for Keys<'a, K, V> { type Item = &'a K; fn next(&mut self) -> Option<&'a K> { todo!() } }
impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> { fn len(&self) -> usize { todo!() } }
impl<'a, K, V> Iterator for Values<'a, K, V> { type Item = &'a V; fn next(&mut self) -> Option<&'a V> { todo!() } }
impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> { fn len(&self) -> usize { todo!() } }
impl<'a, K, V> Iterator for ValuesMut<'a, K, V> { type Item = &'a mut V; fn next(&mut self) -> Option<&'a mut V> { todo!() } }
impl<K, V> Iterator for IntoKeys<K, V> { type Item = K; fn next(&mut self) -> Option<K> { todo!() } }
impl<K, V> Iterator for IntoValues<K, V> { type Item = V; fn next(&mut self) -> Option<V> { todo!() } }
impl<'a, K, V> Iterator for Drain<'a, K, V> { type Item = (K, V); fn next(&mut self) -> Option<(K, V)> { todo!() } }

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> { todo!() }
}
impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;
    fn into_iter(self) -> std::collections::hash_map::Iter<'a, K, V> { todo!() }
}
impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = std::collections::hash_map::IterMut<'a, K, V>;
    fn into_iter(self) -> std::collections::hash_map::IterMut<'a, K, V> { todo!() }
}

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for HashMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> HashMap<K, V, S> { todo!() }
}
impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for HashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) { todo!() }
}

impl<K: Clone, V: Clone, S: Clone> Clone for HashMap<K, V, S> { fn clone(&self) -> HashMap<K, V, S> { todo!() } }
impl<K: Debug, V: Debug, S> Debug for HashMap<K, V, S> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<K, V, S: Default> Default for HashMap<K, V, S> { fn default() -> HashMap<K, V, S> { todo!() } }
impl<K: Eq + Hash, V: PartialEq<V>, S: BuildHasher> PartialEq for HashMap<K, V, S> { fn eq(&self, other: &HashMap<K, V, S>) -> bool { todo!() } }
impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for HashMap<K, V, S> {}
