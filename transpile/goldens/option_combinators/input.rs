// An `Option` combinator is written as the test it is, and a test reads the
// value it tests and then reads it AGAIN to hand it on — where Rust reads it
// once. `subscriptions.remove(id).ok_or(..)` removed the entry, threw it away,
// removed nothing the second time and answered `Err`, with the removed entry
// leaked; `map_or(0, f) == 0` came out `x != null ? f(x) : 0 === 0`, which
// JavaScript reads as `x != null ? f(x) : (0 === 0)` — so it answered `true`
// for every `None`.

use std::collections::HashMap;

pub struct Entry {
    pub weight: u32,
}

pub struct Registry {
    pub entries: HashMap<u32, Entry>,
    pub calls: u32,
}

impl Registry {
    pub fn new() -> Self {
        Registry { entries: HashMap::new(), calls: 0 }
    }

    pub fn put(&mut self, id: u32, weight: u32) {
        self.entries.insert(id, Entry { weight });
    }

    /// The receiver has an effect: `remove` takes the entry OUT. Reading it
    /// twice removed it, discarded what it gave back, and then removed nothing.
    pub fn take(&mut self, id: u32) -> Result<Entry, String> {
        self.calls += 1;
        self.entries.remove(&id).ok_or("no entry".to_string())
    }

    /// `map_or`'s answer is compared, so the whole ternary has to be the thing
    /// compared.
    pub fn weightless(&self, id: u32) -> bool {
        self.entries.get(&id).map_or(0, |e| e.weight) == 0
    }

    /// `map` over a place is a place: nothing is named that need not be.
    pub fn weight_of(&self, id: u32) -> Option<u32> {
        self.entries.get(&id).map(|e| e.weight)
    }

    /// `and_then` chained onto a receiver the engine cannot call a place.
    pub fn heavy_weight(&self, id: u32) -> Option<u32> {
        self.entries.get(&id).and_then(|e| if e.weight > 2 { Some(e.weight) } else { None })
    }

    /// `is_some_and` and `ok_or_else`: the test reads the receiver once, and
    /// the `_else` form's closure is still called only where it is needed.
    pub fn is_heavy(&self, id: u32) -> bool {
        self.entries.get(&id).is_some_and(|e| e.weight > 2)
    }

    pub fn weight_or_fail(&self, id: u32) -> Result<u32, String> {
        self.entries.get(&id).map(|e| e.weight).ok_or_else(|| format!("no {}", id))
    }
}
