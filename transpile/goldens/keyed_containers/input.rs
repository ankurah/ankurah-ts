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
