// A derived `equals` compares the fields the type declares, and HOW two values
// of a field are the same depends on what that field is. A `Uint8Array`, a
// `HashMap`, a `HashSet` and an array carry no `equals` of their own, and a
// primitive carries none either — so each has a comparison written out for it.
// That rule used to stop one level down: a `HashMap<String, Vec<u8>>` compared
// its values with `v.equals(_w)`, and proto's `StateBuffers` threw a TypeError
// on two maps of bytes. `OperationSet`, whose values are `Vec<Operation>`, threw
// one too. And it had no TUPLE case at all, though the clone writer beside it
// has had one since the fourth pass: a `(A, B)` field is a JavaScript array and
// `equals` called a method it has not got — live at `core/reactor/update.ts` on
// a `Vec<(QueryId, MembershipChange)>` and at `storage-common/types.ts` on an
// `Option<(Vec<Value>, bool)>`, both of which the clone writer got right on the
// line below.

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

#[derive(PartialEq, Clone)]
pub struct Paired {
    /// A tuple of a primitive and a class: each position by its own rule.
    pub one: (u32, Tag),
    /// A tuple inside a container, and a nullable tuple holding a container.
    pub many: Vec<(String, Tag)>,
    pub maybe: Option<(Vec<Tag>, bool)>,
    /// A ONE-element tuple, which the clone writer used to send to `.clone()`
    /// on an array because it asked for two parts or more.
    pub single: (Tag,),
}

/// A field written as one of the type's own PARAMETERS: `T` is a number in
/// `Holder<u32>` and a class in `Holder<Tag>`, and both `.equals()` and
/// `.clone()` on a number are TypeErrors. The struct writer was told this in
/// the fourth pass and the ENUM writer was not, so `RangeBound<T>`,
/// `ExprOutput<T>`, `FilterResult<R>` and `ItemChange<I>` all compared and
/// copied their payloads with methods the value may not have.
#[derive(PartialEq, Clone)]
pub struct Holder<T> {
    pub one: T,
    pub many: Vec<T>,
}

#[derive(PartialEq, Clone)]
pub enum Slot<T> {
    Empty,
    One(T),
    Many(Vec<T>),
}
