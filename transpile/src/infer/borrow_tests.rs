//! What a borrow may stand for where a declaration wants something else.
//!
//! Rust's deref coercion takes references OFF a value; it never changes what
//! the target owns or how uniquely it borrows. Each of these is a program Rust
//! rejects, and the port's own cost for accepting it is named in the test.

use crate::testing::Fixture;

use super::mismatch_tests::contradictions;

/// A `&Tag` bound to a by-value `Tag` parameter: the callee releases what the
/// caller still holds, which the runtime reports as a double drop.
#[test]
fn a_borrow_handed_where_a_value_is_owned_reports() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u64 }\n\
         pub fn take_owned(value: Tag) -> u64 { value.n }\n\
         pub fn go(value: &Tag) -> u64 { take_owned(value) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

/// The same through a unique borrow, which is what the spec's reference bullet
/// used to call Rust's deref coercion.
#[test]
fn a_unique_borrow_handed_where_a_value_is_owned_reports() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u64 }\n\
         pub fn take_owned(values: Vec<Tag>) -> usize { values.len() }\n\
         pub fn go(values: &mut Vec<Tag>) -> usize { take_owned(values) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

/// A shared borrow where a unique one is declared: the callee's write has
/// nowhere to land.
#[test]
fn a_shared_borrow_handed_where_a_unique_one_is_declared_reports() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u64 }\n\
         pub fn take_unique(value: &mut Tag) -> u64 { value.n }\n\
         pub fn go(value: &Tag) -> u64 { take_unique(value) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

/// The other direction is Rust's own reborrow and says nothing.
#[test]
fn a_unique_borrow_handed_where_a_shared_one_is_declared_is_a_reborrow() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u64 }\n\
         pub fn take_shared(value: &Tag) -> u64 { value.n }\n\
         pub fn go(value: &mut Tag) -> u64 { take_shared(value) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}

/// A value carrying MORE references than the declaration wants still meets it:
/// that is the coercion Rust performs.
#[test]
fn a_surplus_reference_on_the_value_is_taken_off() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u64 }\n\
         pub fn take_one(value: &Tag) -> u64 { value.n }\n\
         pub fn go(value: &&Tag) -> u64 { take_one(value) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}

/// Where the target owns nothing, the port writes the same code either way, so
/// the borrow is not read: the engine's own reference readings are not exact
/// enough to report a program that costs nothing.
#[test]
fn a_borrow_of_something_with_nothing_to_release_is_not_read() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub fn take_owned(value: u8) -> u8 { value }\n\
         pub fn go(value: &u8) -> u8 { take_owned(value) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}

/// A comparison hands nothing over, and Rust compares through a borrow either
/// side carries.
#[test]
fn a_comparison_reads_through_a_borrow() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub fn same(left: String, right: &str) -> bool { left == right }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}
