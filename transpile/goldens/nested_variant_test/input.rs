// A pattern that takes NO name out of a value can still ask a question of it,
// and the two were read as one: `Some(Status::Requested(_, _))` was written as
// a bare `!= null`, so core's `client_relay` answered "requested" for every
// status that was there at all. `Wrap::Inner(Status::Requested(_, _))` lost its
// inner test the same way.
//
// And a derived `hash()` joined its parts with a separator a `String` field can
// contain, so `Pair("x|s:y", "z")` and `Pair("x", "y|s:z")` hashed alike: two
// different keys in one bucket.
pub struct Id {
    pub n: u32,
}

pub enum Status {
    Requested(Id, u32),
    Established(Id, u32),
    Idle,
}

pub enum Wrap {
    Inner(Status),
    Other,
}

pub fn is_requested(s: &Option<Status>) -> bool {
    match s {
        Some(Status::Requested(_, _)) => true,
        _ => false,
    }
}

pub fn wraps_requested(w: &Wrap) -> bool {
    match w {
        Wrap::Inner(Status::Requested(_, _)) => true,
        _ => false,
    }
}

/// The control: a nameless payload that asks nothing still asks nothing.
pub fn is_anything(s: &Option<Status>) -> bool {
    match s {
        Some(_) => true,
        None => false,
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct Pair {
    pub a: String,
    pub b: String,
}
