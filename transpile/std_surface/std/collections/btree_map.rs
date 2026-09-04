//! `std::collections::btree_map`

pub struct BTreeMap<K, V>;

impl<K, V> BTreeMap<K, V> {
    pub fn new() -> BTreeMap<K, V> { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn clear(&mut self) { todo!() }

    pub fn keys(&self) -> Keys<'_, K, V> { todo!() }
    pub fn values(&self) -> Values<'_, K, V> { todo!() }
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> { todo!() }
    pub fn into_keys(self) -> IntoKeys<K, V> { todo!() }
    pub fn into_values(self) -> IntoValues<K, V> { todo!() }
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, K, V> { todo!() }
    pub fn iter_mut(&mut self) -> std::collections::btree_map::IterMut<'_, K, V> { todo!() }
}

impl<K: Ord, V> BTreeMap<K, V> {
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> { todo!() }
    pub fn insert(&mut self, key: K, value: V) -> Option<V> { todo!() }
    pub fn append(&mut self, other: &mut BTreeMap<K, V>) { todo!() }
    pub fn split_off<Q: ?Sized + Ord>(&mut self, key: &Q) -> BTreeMap<K, V> where K: Borrow<Q> { todo!() }
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, f: F) { todo!() }

    pub fn get<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&V> where K: Borrow<Q> { todo!() }
    pub fn get_mut<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<&mut V> where K: Borrow<Q> { todo!() }
    pub fn get_key_value<Q: ?Sized + Ord>(&self, key: &Q) -> Option<(&K, &V)> where K: Borrow<Q> { todo!() }
    pub fn contains_key<Q: ?Sized + Ord>(&self, key: &Q) -> bool where K: Borrow<Q> { todo!() }
    pub fn remove<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<V> where K: Borrow<Q> { todo!() }
    pub fn remove_entry<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<(K, V)> where K: Borrow<Q> { todo!() }

    pub fn first_key_value(&self) -> Option<(&K, &V)> { todo!() }
    pub fn last_key_value(&self) -> Option<(&K, &V)> { todo!() }
    pub fn pop_first(&mut self) -> Option<(K, V)> { todo!() }
    pub fn pop_last(&mut self) -> Option<(K, V)> { todo!() }
    pub fn range<T: ?Sized + Ord, R: RangeBounds<T>>(&self, range: R) -> Range<'_, K, V> where K: Borrow<T> { todo!() }
    pub fn range_mut<T: ?Sized + Ord, R: RangeBounds<T>>(&mut self, range: R) -> RangeMut<'_, K, V> where K: Borrow<T> { todo!() }
}

pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K: Ord, V> Entry<'a, K, V> {
    pub fn or_insert(self, default: V) -> &'a mut V { todo!() }
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V { todo!() }
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V { todo!() }
    pub fn or_default(self) -> &'a mut V where V: Default { todo!() }
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Entry<'a, K, V> { todo!() }
    pub fn key(&self) -> &K { todo!() }
}

pub struct OccupiedEntry<'a, K, V>;

impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K { todo!() }
    pub fn get(&self) -> &V { todo!() }
    pub fn get_mut(&mut self) -> &mut V { todo!() }
    pub fn into_mut(self) -> &'a mut V { todo!() }
    pub fn insert(&mut self, value: V) -> V { todo!() }
    pub fn remove(self) -> V { todo!() }
    pub fn remove_entry(self) -> (K, V) { todo!() }
}

pub struct VacantEntry<'a, K, V>;

impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
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
pub struct Range<'a, K, V>;
pub struct RangeMut<'a, K, V>;

impl<'a, K, V> Iterator for Iter<'a, K, V> { type Item = (&'a K, &'a V); fn next(&mut self) -> Option<(&'a K, &'a V)> { todo!() } }
impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> { fn next_back(&mut self) -> Option<(&'a K, &'a V)> { todo!() } }
impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> { fn len(&self) -> usize { todo!() } }
impl<'a, K, V> Clone for Iter<'a, K, V> { fn clone(&self) -> Iter<'a, K, V> { todo!() } }
impl<'a, K, V> Iterator for IterMut<'a, K, V> { type Item = (&'a K, &'a mut V); fn next(&mut self) -> Option<(&'a K, &'a mut V)> { todo!() } }
impl<K, V> Iterator for IntoIter<K, V> { type Item = (K, V); fn next(&mut self) -> Option<(K, V)> { todo!() } }
impl<K, V> DoubleEndedIterator for IntoIter<K, V> { fn next_back(&mut self) -> Option<(K, V)> { todo!() } }
impl<'a, K, V> Iterator for Keys<'a, K, V> { type Item = &'a K; fn next(&mut self) -> Option<&'a K> { todo!() } }
impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> { fn next_back(&mut self) -> Option<&'a K> { todo!() } }
impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> { fn len(&self) -> usize { todo!() } }
impl<'a, K, V> Iterator for Values<'a, K, V> { type Item = &'a V; fn next(&mut self) -> Option<&'a V> { todo!() } }
impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> { fn len(&self) -> usize { todo!() } }
impl<'a, K, V> Iterator for ValuesMut<'a, K, V> { type Item = &'a mut V; fn next(&mut self) -> Option<&'a mut V> { todo!() } }
impl<K, V> Iterator for IntoKeys<K, V> { type Item = K; fn next(&mut self) -> Option<K> { todo!() } }
impl<K, V> Iterator for IntoValues<K, V> { type Item = V; fn next(&mut self) -> Option<V> { todo!() } }
impl<'a, K, V> Iterator for Range<'a, K, V> { type Item = (&'a K, &'a V); fn next(&mut self) -> Option<(&'a K, &'a V)> { todo!() } }
impl<'a, K, V> DoubleEndedIterator for Range<'a, K, V> { fn next_back(&mut self) -> Option<(&'a K, &'a V)> { todo!() } }
impl<'a, K, V> Iterator for RangeMut<'a, K, V> { type Item = (&'a K, &'a mut V); fn next(&mut self) -> Option<(&'a K, &'a mut V)> { todo!() } }

impl<K, V> IntoIterator for BTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> { todo!() }
}
impl<'a, K, V> IntoIterator for &'a BTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::btree_map::Iter<'a, K, V>;
    fn into_iter(self) -> std::collections::btree_map::Iter<'a, K, V> { todo!() }
}
impl<'a, K, V> IntoIterator for &'a mut BTreeMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = std::collections::btree_map::IterMut<'a, K, V>;
    fn into_iter(self) -> std::collections::btree_map::IterMut<'a, K, V> { todo!() }
}

impl<K: Ord, V> FromIterator<(K, V)> for BTreeMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> BTreeMap<K, V> { todo!() }
}
impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) { todo!() }
}

impl<K: Clone, V: Clone> Clone for BTreeMap<K, V> { fn clone(&self) -> BTreeMap<K, V> { todo!() } }
impl<K: Debug, V: Debug> Debug for BTreeMap<K, V> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<K, V> Default for BTreeMap<K, V> { fn default() -> BTreeMap<K, V> { todo!() } }
impl<K: PartialEq<K>, V: PartialEq<V>> PartialEq for BTreeMap<K, V> { fn eq(&self, other: &BTreeMap<K, V>) -> bool { todo!() } }
impl<K: Eq, V: Eq> Eq for BTreeMap<K, V> {}
impl<K: Ord, V: Ord> Ord for BTreeMap<K, V> { fn cmp(&self, other: &BTreeMap<K, V>) -> std::cmp::Ordering { todo!() } }
