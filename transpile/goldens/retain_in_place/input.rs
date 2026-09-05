// `retain` keeps what the predicate accepts and DROPS the rest, in place.
//
// Two defects met here. A `Vec::retain` was emitted as a `/* TODO: retain */`
// comment and a `filter` whose answer nobody read, so the vector was unchanged
// and nothing was dropped. A `HashMap::retain` wrote its predicate unparenthesised
// in front of the argument list — `(k, v) => body(_k, _v)` — and an arrow's body
// extends as far as it can, so the call became part of the body and what the `!`
// tested was the arrow itself, an object and always truthy: nothing was ever
// deleted, and `!entry.markedForRemoval(_k, _v)` called a boolean.
//
// Three more met here since. The predicate was interpolated INSIDE the loop, so
// a `move` closure was built once per element and an `OwnedClosure` — which is
// not callable as a function — threw on the first. `retain` takes its predicate
// by value and Rust drops it when the call ends, and nothing released it. And
// the array was truncated only on normal completion, so a predicate that threw
// left the already-dropped elements in place and the kept ones duplicated: the
// next cascade dropped them twice.

use std::collections::HashMap;

pub struct Item {
    pub n: u32,
}

pub struct Bag {
    pub items: Vec<Item>,
    pub flags: HashMap<String, bool>,
}

pub struct Gate {
    pub least: u32,
}

impl Bag {
    pub fn keep_over(&mut self, least: u32) {
        self.items.retain(|item| item.n >= least);
    }

    pub fn keep_set(&mut self) {
        self.flags.retain(|_, on| *on);
    }

    /// A `move` predicate that captures something with drop glue: an
    /// `OwnedClosure`, which is reached through `invokeRef` and released when
    /// the call ends — once, not once per element.
    pub fn keep_over_gate(&mut self, gate: Gate) {
        self.items.retain(move |item| item.n >= gate.least);
    }

    /// A predicate that panics part way through. Rust leaves the vector valid:
    /// what it accepted stays, what it rejected is gone, and everything it
    /// never reached is kept.
    pub fn keep_until_zero(&mut self) {
        self.items.retain(|item| {
            if item.n == 0 {
                panic!("zero");
            }
            item.n > 1
        });
    }
}
