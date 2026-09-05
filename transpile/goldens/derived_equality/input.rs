// A derived `equals` compares the fields the type declares, and HOW two values
// of a field are the same depends on what that field is. A `Uint8Array`, a
// `HashMap`, a `HashSet` and an array carry no `equals` of their own, and a
// primitive carries none either — so each has a comparison written out for it.
// That rule used to stop one level down: a `HashMap<String, Vec<u8>>` compared
// its values with `v.equals(_w)`, and proto's `StateBuffers` threw a TypeError
// on two maps of bytes. `OperationSet`, whose values are `Vec<Operation>`, threw
// one too.

use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Tag {
    pub name: String,
}

#[derive(PartialEq, Clone)]
pub struct Buffers {
    pub parts: HashMap<String, Vec<u8>>,
}

#[derive(PartialEq, Clone)]
pub struct Groups {
    pub members: HashMap<String, Vec<Tag>>,
}

#[derive(PartialEq, Clone)]
pub struct Marked {
    pub tags: HashSet<Tag>,
}

#[derive(PartialEq, Clone)]
pub struct Maybe {
    pub tag: Option<Tag>,
    pub count: Option<u32>,
}

#[derive(PartialEq, Clone)]
pub struct Nested {
    pub rows: Vec<Vec<u8>>,
}

/// A nullable INSIDE a container: the field itself is not one, so the guard the
/// field writer puts around a `T | null` field is not there to help.
#[derive(PartialEq, Clone)]
pub struct Sparse {
    pub slots: HashMap<String, Option<Tag>>,
}
