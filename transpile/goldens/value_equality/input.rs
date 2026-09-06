// `==` compares CONTENTS in Rust and identity in JavaScript.
//
// Every route to a `PartialEq` impl that the operator table could not take used
// to leave `===` standing, and `===` between two objects, two arrays or two
// byte buffers is false for every pair that is not the same value. Eight
// emitted sites were live: `bytes == [0u8; 16]` in `collatable` (three of
// them), two `BTreeSet`s in `lineage`, `ValueType::of(l) == ValueType::of(r)`
// in `filter`, a `KeySpec` in `resultset`, and `diff == Update::EMPTY_V2` in
// the yjs backend. Each was a branch that could never be taken.
//
// `filter`'s was invisible for a second reason — the `use` that names the type
// is written INSIDE the body, Rust scopes it to that block, and the engine's
// binding table is per module, so neither operand had a type at all. That one
// needs two modules to reproduce and a golden is one file, so it is pinned by
// `a_type_named_by_a_body_use_is_resolved` in `registry/engine_tests.rs`
// instead.

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Small,
    Large,
}

pub struct Tag {
    pub name: String,
}

/// Two byte buffers. `===` between them was identity: always false.
pub fn is_zero(bytes: &Vec<u8>) -> bool {
    *bytes == vec![0u8, 0, 0, 0]
}

/// Two sets. The runtime container compares by contents and ignores order.
pub fn same_members(a: &HashSet<u32>, b: &HashSet<u32>) -> bool {
    a == b
}

/// Two fieldless enum values, each freshly built.
pub fn same_kind(l: Kind, r: Kind) -> bool {
    l == r
}

/// And the negation of the same question, over a type with a field.
pub fn different_tag(l: &Tag, r: &Tag) -> bool {
    l.name != r.name
}
