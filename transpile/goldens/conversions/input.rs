//! The conversion family: `.into()`, `T::from(..)`, `to_string`, `to_owned`,
//! and `as` between numeric types.
//!
//! Each of these is a value conversion in Rust and nothing at all in
//! TypeScript's `as`, so what the port writes is either the impl Rust selects,
//! the arithmetic JavaScript needs, or the value itself where the two types are
//! one here.

pub struct Tag {
    pub label: u32,
}

pub struct Name {
    pub text: String,
}

impl From<Tag> for Name {
    fn from(tag: Tag) -> Name {
        Name { text: tag.label.to_string() }
    }
}

/// `.into()` with the target named by the position it stands in.
pub fn named(tag: Tag) -> Name {
    tag.into()
}

/// The same conversion written the other way round.
pub fn from_call(tag: Tag) -> Name {
    Name::from(tag)
}

/// `to_string` on a string is the string: `String` and `&str` are one type in
/// the port, so the call had nothing to do.
pub fn owned(raw: &str) -> String {
    raw.to_string()
}

/// A widening that crosses from `number` to `bigint`, which JavaScript will not
/// do on its own.
pub fn widen(n: u32) -> u64 {
    n as u64
}

/// A narrowing that keeps the low bits, as Rust's `as` does.
pub fn narrow(n: u64) -> u32 {
    n as u32
}

/// A float truncated towards zero on its way to an integer.
pub fn truncate(f: f64) -> i32 {
    f as i32
}
