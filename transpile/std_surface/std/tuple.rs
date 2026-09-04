//! Trait impls on tuples — `core/src/tuple.rs` in std's own layout.
//!
//! std generates these by macro for arities 1 through 12. Arities 1 through 6
//! are written out here: the corpus's widest tuple is the planner's
//! `(String, Vec<(ComparisonOperator, Value)>)` nesting and `(&K, &V)` map
//! items, and each unused arity is 8 impls of parse time for nothing. A
//! 7-tuple in a future ankurah version is a diagnostic, not a wrong answer.
//!
//! `Hash` and `Eq` matter most: a composite map or set key is a tuple, and
//! without these `HashMap<(CollectionId, EntityId), _>` has no key bound.

// ── 1-tuple ──
impl<T0: Clone> Clone for (T0,) { fn clone(&self) -> (T0,) { todo!() } }
impl<T0: Copy> Copy for (T0,) {}
impl<T0: PartialEq> PartialEq for (T0,) { fn eq(&self, other: &(T0,)) -> bool { todo!() } }
impl<T0: Eq> Eq for (T0,) {}
impl<T0: PartialOrd> PartialOrd for (T0,) { fn partial_cmp(&self, other: &(T0,)) -> Option<std::cmp::Ordering> { todo!() } }
impl<T0: Ord> Ord for (T0,) { fn cmp(&self, other: &(T0,)) -> std::cmp::Ordering { todo!() } }
impl<T0: Hash> Hash for (T0,) { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T0: Default> Default for (T0,) { fn default() -> (T0,) { todo!() } }
impl<T0: Debug> Debug for (T0,) { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T0> Tuple for (T0,) {}

// ── 2-tuple ──
impl<T0: Clone, T1: Clone> Clone for (T0, T1) { fn clone(&self) -> (T0, T1) { todo!() } }
impl<T0: Copy, T1: Copy> Copy for (T0, T1) {}
impl<T0: PartialEq, T1: PartialEq> PartialEq for (T0, T1) { fn eq(&self, other: &(T0, T1)) -> bool { todo!() } }
impl<T0: Eq, T1: Eq> Eq for (T0, T1) {}
impl<T0: PartialOrd, T1: PartialOrd> PartialOrd for (T0, T1) { fn partial_cmp(&self, other: &(T0, T1)) -> Option<std::cmp::Ordering> { todo!() } }
impl<T0: Ord, T1: Ord> Ord for (T0, T1) { fn cmp(&self, other: &(T0, T1)) -> std::cmp::Ordering { todo!() } }
impl<T0: Hash, T1: Hash> Hash for (T0, T1) { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T0: Default, T1: Default> Default for (T0, T1) { fn default() -> (T0, T1) { todo!() } }
impl<T0: Debug, T1: Debug> Debug for (T0, T1) { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T0, T1> Tuple for (T0, T1) {}

// ── 3-tuple ──
impl<T0: Clone, T1: Clone, T2: Clone> Clone for (T0, T1, T2) { fn clone(&self) -> (T0, T1, T2) { todo!() } }
impl<T0: Copy, T1: Copy, T2: Copy> Copy for (T0, T1, T2) {}
impl<T0: PartialEq, T1: PartialEq, T2: PartialEq> PartialEq for (T0, T1, T2) { fn eq(&self, other: &(T0, T1, T2)) -> bool { todo!() } }
impl<T0: Eq, T1: Eq, T2: Eq> Eq for (T0, T1, T2) {}
impl<T0: PartialOrd, T1: PartialOrd, T2: PartialOrd> PartialOrd for (T0, T1, T2) { fn partial_cmp(&self, other: &(T0, T1, T2)) -> Option<std::cmp::Ordering> { todo!() } }
impl<T0: Ord, T1: Ord, T2: Ord> Ord for (T0, T1, T2) { fn cmp(&self, other: &(T0, T1, T2)) -> std::cmp::Ordering { todo!() } }
impl<T0: Hash, T1: Hash, T2: Hash> Hash for (T0, T1, T2) { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T0: Default, T1: Default, T2: Default> Default for (T0, T1, T2) { fn default() -> (T0, T1, T2) { todo!() } }
impl<T0: Debug, T1: Debug, T2: Debug> Debug for (T0, T1, T2) { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T0, T1, T2> Tuple for (T0, T1, T2) {}

// ── 4-tuple ──
impl<T0: Clone, T1: Clone, T2: Clone, T3: Clone> Clone for (T0, T1, T2, T3) { fn clone(&self) -> (T0, T1, T2, T3) { todo!() } }
impl<T0: Copy, T1: Copy, T2: Copy, T3: Copy> Copy for (T0, T1, T2, T3) {}
impl<T0: PartialEq, T1: PartialEq, T2: PartialEq, T3: PartialEq> PartialEq for (T0, T1, T2, T3) { fn eq(&self, other: &(T0, T1, T2, T3)) -> bool { todo!() } }
impl<T0: Eq, T1: Eq, T2: Eq, T3: Eq> Eq for (T0, T1, T2, T3) {}
impl<T0: PartialOrd, T1: PartialOrd, T2: PartialOrd, T3: PartialOrd> PartialOrd for (T0, T1, T2, T3) { fn partial_cmp(&self, other: &(T0, T1, T2, T3)) -> Option<std::cmp::Ordering> { todo!() } }
impl<T0: Ord, T1: Ord, T2: Ord, T3: Ord> Ord for (T0, T1, T2, T3) { fn cmp(&self, other: &(T0, T1, T2, T3)) -> std::cmp::Ordering { todo!() } }
impl<T0: Hash, T1: Hash, T2: Hash, T3: Hash> Hash for (T0, T1, T2, T3) { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T0: Default, T1: Default, T2: Default, T3: Default> Default for (T0, T1, T2, T3) { fn default() -> (T0, T1, T2, T3) { todo!() } }
impl<T0: Debug, T1: Debug, T2: Debug, T3: Debug> Debug for (T0, T1, T2, T3) { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T0, T1, T2, T3> Tuple for (T0, T1, T2, T3) {}

// ── 5-tuple ──
impl<T0: Clone, T1: Clone, T2: Clone, T3: Clone, T4: Clone> Clone for (T0, T1, T2, T3, T4) { fn clone(&self) -> (T0, T1, T2, T3, T4) { todo!() } }
impl<T0: Copy, T1: Copy, T2: Copy, T3: Copy, T4: Copy> Copy for (T0, T1, T2, T3, T4) {}
impl<T0: PartialEq, T1: PartialEq, T2: PartialEq, T3: PartialEq, T4: PartialEq> PartialEq for (T0, T1, T2, T3, T4) { fn eq(&self, other: &(T0, T1, T2, T3, T4)) -> bool { todo!() } }
impl<T0: Eq, T1: Eq, T2: Eq, T3: Eq, T4: Eq> Eq for (T0, T1, T2, T3, T4) {}
impl<T0: PartialOrd, T1: PartialOrd, T2: PartialOrd, T3: PartialOrd, T4: PartialOrd> PartialOrd for (T0, T1, T2, T3, T4) { fn partial_cmp(&self, other: &(T0, T1, T2, T3, T4)) -> Option<std::cmp::Ordering> { todo!() } }
impl<T0: Ord, T1: Ord, T2: Ord, T3: Ord, T4: Ord> Ord for (T0, T1, T2, T3, T4) { fn cmp(&self, other: &(T0, T1, T2, T3, T4)) -> std::cmp::Ordering { todo!() } }
impl<T0: Hash, T1: Hash, T2: Hash, T3: Hash, T4: Hash> Hash for (T0, T1, T2, T3, T4) { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T0: Default, T1: Default, T2: Default, T3: Default, T4: Default> Default for (T0, T1, T2, T3, T4) { fn default() -> (T0, T1, T2, T3, T4) { todo!() } }
impl<T0: Debug, T1: Debug, T2: Debug, T3: Debug, T4: Debug> Debug for (T0, T1, T2, T3, T4) { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T0, T1, T2, T3, T4> Tuple for (T0, T1, T2, T3, T4) {}

// ── 6-tuple ──
impl<T0: Clone, T1: Clone, T2: Clone, T3: Clone, T4: Clone, T5: Clone> Clone for (T0, T1, T2, T3, T4, T5) { fn clone(&self) -> (T0, T1, T2, T3, T4, T5) { todo!() } }
impl<T0: Copy, T1: Copy, T2: Copy, T3: Copy, T4: Copy, T5: Copy> Copy for (T0, T1, T2, T3, T4, T5) {}
impl<T0: PartialEq, T1: PartialEq, T2: PartialEq, T3: PartialEq, T4: PartialEq, T5: PartialEq> PartialEq for (T0, T1, T2, T3, T4, T5) { fn eq(&self, other: &(T0, T1, T2, T3, T4, T5)) -> bool { todo!() } }
impl<T0: Eq, T1: Eq, T2: Eq, T3: Eq, T4: Eq, T5: Eq> Eq for (T0, T1, T2, T3, T4, T5) {}
impl<T0: PartialOrd, T1: PartialOrd, T2: PartialOrd, T3: PartialOrd, T4: PartialOrd, T5: PartialOrd> PartialOrd for (T0, T1, T2, T3, T4, T5) { fn partial_cmp(&self, other: &(T0, T1, T2, T3, T4, T5)) -> Option<std::cmp::Ordering> { todo!() } }
impl<T0: Ord, T1: Ord, T2: Ord, T3: Ord, T4: Ord, T5: Ord> Ord for (T0, T1, T2, T3, T4, T5) { fn cmp(&self, other: &(T0, T1, T2, T3, T4, T5)) -> std::cmp::Ordering { todo!() } }
impl<T0: Hash, T1: Hash, T2: Hash, T3: Hash, T4: Hash, T5: Hash> Hash for (T0, T1, T2, T3, T4, T5) { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T0: Default, T1: Default, T2: Default, T3: Default, T4: Default, T5: Default> Default for (T0, T1, T2, T3, T4, T5) { fn default() -> (T0, T1, T2, T3, T4, T5) { todo!() } }
impl<T0: Debug, T1: Debug, T2: Debug, T3: Debug, T4: Debug, T5: Debug> Debug for (T0, T1, T2, T3, T4, T5) { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T0, T1, T2, T3, T4, T5> Tuple for (T0, T1, T2, T3, T4, T5) {}

// The unit type is the 0-tuple.
impl Clone for () { fn clone(&self) -> () { todo!() } }
impl Copy for () {}
impl PartialEq for () { fn eq(&self, other: &()) -> bool { todo!() } }
impl Eq for () {}
impl PartialOrd for () { fn partial_cmp(&self, other: &()) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for () { fn cmp(&self, other: &()) -> std::cmp::Ordering { todo!() } }
impl Hash for () { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Tuple for () {}
