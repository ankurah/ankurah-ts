//! Names the port cannot write as they stand.
//!
//! A module-level function is a BINDING, and a binding is where JavaScript
//! refuses a reserved word: `pub fn with(..)` emitted `export function with`,
//! which TypeScript will not take, while every caller already wrote `with_`.
//! `pub fn r#in(..)` was worse — the raw-identifier marker is Rust syntax, and
//! `export function r#in` is a line no JavaScript engine parses at all. One
//! escaping rule, so a declaration and every reference to it agree. A METHOD is
//! not this position: `obj.delete` and a `with` on a class are both legal, and
//! nothing renames a member.
//!
//! And `unsupported` is the one name the PORT writes into a body without the
//! source asking for it: the R12 hole calls it. A crate that declares its own
//! shadowed the helper — base's was not even imported, because the import list
//! is written from what the emitted text names — so every hole in that file
//! answered a value instead of throwing.

use std::collections::BinaryHeap;

/// A user function that spells the hole helper.
pub fn unsupported(_what: &str) -> u32 {
    7
}

/// A reserved word, and a raw identifier that is one.
pub fn with(n: u32) -> u32 {
    n + 1
}

pub fn r#in(n: u32) -> u32 {
    n + 2
}

pub fn calls_them(n: u32) -> u32 {
    with(n) + r#in(n) + unsupported("nothing")
}

/// And a real hole in the same file, which has to throw.
pub fn refuses(xs: Vec<u32>) -> u32 {
    let _h: BinaryHeap<u32> = xs.into_iter().collect();
    0
}

pub struct Holder {
    pub n: u32,
}

impl Holder {
    /// A METHOD called `with` is a property, and stays as it is.
    pub fn with(&self, extra: u32) -> u32 {
        self.n + extra
    }
}
