// Every way the port builds a map or a set goes through the RUNTIME containers,
// which hash a key by its `hash()` and compare by its `equals()`. Written as
// JavaScript's `Map` and `Set` — which compare by identity — a key read back off
// the wire matched nothing, and `HashMap::from` named a static that did not
// exist at all.
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub name: String,
}

#[derive(Clone, Default)]
pub struct Bag {
    pub named: HashMap<Key, u32>,
    pub tags: HashSet<Key>,
}

pub fn built() -> HashMap<Key, u32> {
    HashMap::from([(Key { name: "a".to_string() }, 1u32)])
}

pub fn tagged() -> HashSet<Key> {
    HashSet::from([Key { name: "a".to_string() }])
}

pub fn ordered() -> BTreeMap<Key, u32> {
    BTreeMap::from([(Key { name: "a".to_string() }, 1u32)])
}

/// `*map.entry(k).or_insert(0) += 1` is how the corpus counts, and it has to
/// read the place ONCE: written twice, the entry was made twice and the key
/// cloned twice, and the second clone leaked.
pub fn counted(words: &Vec<Key>) -> HashMap<Key, u32> {
    let mut counts: HashMap<Key, u32> = HashMap::new();
    for w in words {
        *counts.entry(w.clone()).or_insert(0) += 1;
    }
    counts
}

/// R1: a finisher bound to a NAME is the write-through slot itself, not the
/// value in it. Bound to the value, `*slot += 1` wrote `slot.value` on a
/// number, and the count never reached the map.
pub fn counted_by_name(words: &Vec<Key>) -> HashMap<Key, u32> {
    let mut counts: HashMap<Key, u32> = HashMap::new();
    for w in words {
        let slot = counts.entry(w.clone()).or_insert(0);
        *slot += 1;
    }
    counts
}

/// The three ways of finishing an entry that the counter above does not use.
/// Each reads the place as a VALUE: `or_insert` answers a `&mut V`, and
/// `.push(..)` is a call on the `Vec` it points at, which the runtime's
/// write-through `Slot` has no method of its own for.
pub struct Lists {
    pub by_name: HashMap<Key, Vec<u32>>,
    pub ordered: BTreeMap<String, Vec<u32>>,
}

impl Lists {
    pub fn new() -> Self {
        Lists { by_name: HashMap::new(), ordered: BTreeMap::new() }
    }

    pub fn push_default(&mut self, k: Key, v: u32) {
        self.by_name.entry(k).or_default().push(v);
    }

    pub fn push_insert(&mut self, k: Key, v: u32) {
        self.by_name.entry(k).or_insert(Vec::new()).push(v);
    }

    pub fn push_with(&mut self, k: Key, v: u32) {
        self.by_name.entry(k).or_insert_with(|| Vec::new()).push(v);
    }

    /// A `BTreeMap` receiver. The value type an `or_default()` needs a thunk
    /// for is read off `btree_map::Entry` as well as off `hash_map::Entry` —
    /// read off only the first, this emitted `orDefault()` with no thunk, and
    /// `orDefault(undefined)` invokes `undefined` on the first unseen key.
    pub fn push_ordered(&mut self, k: String, v: u32) {
        self.ordered.entry(k).or_default().push(v);
    }

    /// The same slot bound to a name and read as a value: Rust's auto-deref
    /// calls `push` on the `Vec` the slot points at.
    pub fn push_named(&mut self, k: Key, v: u32) {
        let slot = self.by_name.entry(k).or_default();
        slot.push(v);
    }

    pub fn count(&self, k: &Key) -> usize {
        self.by_name.get(k).map_or(0, |v| v.len())
    }

    pub fn ordered_count(&self, k: &String) -> usize {
        self.ordered.get(k).map_or(0, |v| v.len())
    }
}
