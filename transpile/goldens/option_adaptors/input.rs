// The iterator adaptors Rust answers an `Option` with, whose nearest
// JavaScript spelling answers a SENTINEL rather than absence.
//
// `Array.prototype.findIndex` answers `-1`, and `-1 != null` is TRUE, so a
// `position` that found nothing read as PRESENT: the reactor's watcher removal
// ran `entries.splice(-1, 1)` and deleted the last LIVE watcher every time it
// was asked to remove one that had already gone. `find`, `at(-1)` and `[0]`
// answer `undefined`, which `!= null` reads as absent by accident but which is
// not the `null` the port's declared `T | null` promises. `find_map` had no
// lowering at all and was emitted as `xs.findMap(..)`, a method no array has.
// `Array.prototype.reduce` with no initial value THROWS on an empty array
// where Rust's `reduce` answers `None`.
//
// `first_droppable` below is the second answer this golden pins, added with the
// fix for it: a `.iter()` over a BORROWED sequence of droppable elements was
// lifted into a temporary the emitter released, `dropOwned(_t0)` over a vector
// the caller still owns. The glue was read off the iterator's type ARGUMENT
// rather than off its `Iterator::Item`, and `slice::Iter<'a, T>` hands out
// `&'a T`. Live at `storage-common`'s `build_bounds`, where
// `equalities.iter()` over a `&[(String, Value)]` released every `Value` in the
// caller's slice.

pub struct Watchers {
    pub ids: Vec<u32>,
}

impl Watchers {
    pub fn new(ids: Vec<u32>) -> Self {
        Watchers { ids }
    }

    /// The live case. A watcher that has already gone must leave the list
    /// alone.
    pub fn remove(&mut self, id: u32) {
        if let Some(pos) = self.ids.iter().position(|w| *w == id) {
            self.ids.remove(pos);
        }
    }

    /// `rposition` answers the LAST match, not the first.
    pub fn last_at_least(&self, at_least: u32) -> Option<usize> {
        self.ids.iter().rposition(|w| *w >= at_least)
    }
}

/// `find` answers the element or `None`.
pub fn first_over(ns: &Vec<u32>, over: u32) -> Option<u32> {
    ns.iter().find(|n| **n > over).copied()
}

/// `find_map` answers what the closure built for the first `Some` it gets.
pub fn first_label_over(labels: &Vec<String>, over: usize) -> Option<String> {
    labels.iter().find_map(|l| if l.len() > over { Some(l.clone()) } else { None })
}

/// `first` and `last` read the ends of a slice, and both answer `Option`.
pub fn ends(ns: &Vec<u32>) -> (Option<u32>, Option<u32>) {
    (ns.first().copied(), ns.last().copied())
}

/// `reduce` folds with no initial value, and answers `None` for an empty
/// sequence rather than raising.
pub fn total(ns: Vec<u32>) -> Option<u32> {
    ns.into_iter().reduce(|a, b| a + b)
}

/// `max_by_key` keeps the LAST of a tie and `min_by_key` the first — std's own
/// asymmetry, visible whenever the elements carry more than the key reads.
pub fn widest(labels: &Vec<String>) -> Option<String> {
    labels.iter().max_by_key(|l| l.len()).cloned()
}

pub fn narrowest(labels: &Vec<String>) -> Option<String> {
    labels.iter().min_by_key(|l| l.len()).cloned()
}

pub struct Reading {
    pub label: String,
}

impl Reading {
    pub fn new(label: String) -> Self {
        Reading { label }
    }
}

/// `.iter()` over a BORROWED sequence of droppable elements. The iterator hands
/// out `&Reading`, so the array the port spreads it into owns nothing and the
/// caller keeps every element.
pub fn first_droppable(readings: &Vec<Reading>, prefix: &str) -> bool {
    readings.iter().find(|r| r.label.starts_with(prefix)).is_some()
}

/// A range is a VALUE in Rust, and the port has no `Range` type: this used to
/// emit `for (const n of undefined)`, which raises the first time the loop is
/// reached. `Entity::commit`'s retry loop is one of those.
pub fn counted(to: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for n in 0..to {
        out.push(n);
    }
    out
}

/// `rev()` walks the same sequence backwards and leaves the original alone, and
/// `filter_map` keeps what the closure answers `Some` for — written as their
/// own names they were `xs.rev()` and `xs.filterMap(..)`, methods no array has.
pub fn evens_backwards(to: usize) -> Vec<usize> {
    (0..to).rev().filter_map(|n| if n % 2 == 0 { Some(n) } else { None }).collect()
}
