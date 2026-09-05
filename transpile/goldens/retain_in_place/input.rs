// `retain` keeps what the predicate accepts and DROPS the rest, in place.
//
// Two defects met here. A `Vec::retain` was emitted as a `/* TODO: retain */`
// comment and a `filter` whose answer nobody read, so the vector was unchanged
// and nothing was dropped. A `HashMap::retain` wrote its predicate unparenthesised
// in front of the argument list — `(k, v) => body(_k, _v)` — and an arrow's body
// extends as far as it can, so the call became part of the body and what the `!`
// tested was the arrow itself, an object and always truthy: nothing was ever
// deleted, and `!entry.markedForRemoval(_k, _v)` called a boolean.

use std::collections::HashMap;

pub struct Item {
    pub n: u32,
}

pub struct Bag {
    pub items: Vec<Item>,
    pub flags: HashMap<String, bool>,
}

impl Bag {
    pub fn keep_over(&mut self, least: u32) {
        self.items.retain(|item| item.n >= least);
    }

    pub fn keep_set(&mut self) {
        self.flags.retain(|_, on| *on);
    }
}
