//! What a written `&` decides about ownership.
//!
//! The port erases references nearly everywhere, because a TypeScript object
//! reference is what a Rust reference becomes. Two places cannot afford that:
//! the sequence a `for` loop iterates (`IntoIterator for Vec<T>` hands out a
//! `T` the loop releases, `IntoIterator for &Vec<T>` a `&T` it does not) and the
//! value a pattern is matched against (Rust's default binding mode, RFC 2005:
//! a pattern matched against a reference binds by reference). These tests write
//! the same value both ways and read where the releases land.

use crate::testing::Fixture;

const PRELUDE: &str = "\
use std::collections::HashMap;\n\
pub struct Key { pub name: String }\n\
pub struct Cell { pub value: u32 }\n\
";

fn body(rust: &str, method: &str) -> String {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", PRELUDE, rust))]);
    fixture.translated_method("lib.rs", method)
}

#[test]
fn iterating_a_borrowed_map_releases_neither_key_nor_value() {
    let ts = body(
        "pub fn f(map: HashMap<Key, Cell>) -> u32 {\n\
         let mut n = 0u32;\n\
         for (_k, v) in &map { n += v.value; }\n\
         n }",
        "f",
    );
    assert!(
        !ts.contains("_k.drop()") && !ts.contains("v.drop()"),
        "a borrowed map iteration owns neither the key nor the value:\n{ts}"
    );
    // The map itself is still the parameter's, and the parameter is still owned.
    assert!(ts.contains("dropOwned(map)"), "the map is still released:\n{ts}");
}

#[test]
fn iterating_a_map_by_value_releases_the_key_and_the_value() {
    let ts = body(
        "pub fn f(map: HashMap<Key, Cell>) -> u32 {\n\
         let mut n = 0u32;\n\
         for (_k, v) in map { n += v.value; }\n\
         n }",
        "f",
    );
    assert!(
        ts.contains("_k.drop()") && ts.contains("v.drop()"),
        "a by-value map iteration takes the key and the value out:\n{ts}"
    );
}

#[test]
fn iterating_a_borrowed_vec_leaves_the_elements_to_the_vec() {
    let ts = body(
        "pub fn f(v: Vec<Cell>) -> u32 {\n\
         let mut n = 0u32;\n\
         for c in &v { n += c.value; }\n\
         n }",
        "f",
    );
    assert!(
        !ts.contains("c.drop()") && !ts.contains(".slice("),
        "a borrowed vec iteration neither releases elements nor releases a tail:\n{ts}"
    );
    assert!(ts.contains("for (const c of v)"), "it is a plain for-of:\n{ts}");
}

#[test]
fn iterating_a_vec_by_value_still_releases_the_tail() {
    let ts = body(
        "pub fn f(v: Vec<Cell>) -> u32 {\n\
         let mut n = 0u32;\n\
         for c in v { n += c.value; }\n\
         n }",
        "f",
    );
    assert!(ts.contains("c.drop()"), "each element is released:\n{ts}");
    assert!(
        ts.contains(".slice("),
        "the elements the loop never reached are released:\n{ts}"
    );
}

#[test]
fn a_reference_written_through_a_deref_is_still_a_borrow() {
    let ts = body(
        "pub fn f(map: &HashMap<Key, Cell>) -> u32 {\n\
         let mut n = 0u32;\n\
         for (_k, v) in &*map { n += v.value; }\n\
         n }",
        "f",
    );
    assert!(
        !ts.contains("_k.drop()") && !ts.contains("v.drop()"),
        "`&*map` borrows just as `&map` does:\n{ts}"
    );
}

#[test]
fn an_if_let_over_a_reference_releases_nothing() {
    let ts = body(
        "pub struct Holder { pub items: Option<Vec<Cell>> }\n\
         pub fn f(h: &Holder) -> u32 {\n\
         let mut n = 0u32;\n\
         if let Some(items) = &h.items { for c in items { n += c.value; } }\n\
         n }",
        "f",
    );
    assert!(
        !ts.contains("dropOwned(items)") && !ts.contains("items.drop()"),
        "the vector is still the field's:\n{ts}"
    );
}

#[test]
fn an_if_let_over_an_owned_value_still_releases_it() {
    let ts = body(
        "pub struct Holder { pub items: Option<Vec<Cell>> }\n\
         pub fn f(h: Holder) -> u32 {\n\
         let mut n = 0u32;\n\
         if let Some(items) = h.items { for c in items { n += c.value; } }\n\
         n }",
        "f",
    );
    assert!(
        ts.contains("dropOwned(") || ts.contains(".drop()"),
        "the vector was taken out of the struct, so somebody owes a release:\n{ts}"
    );
}

#[test]
fn a_match_over_a_reference_binds_a_borrow() {
    let ts = body(
        "pub enum Held { One(Cell), None }\n\
         pub fn f(h: &Held) -> u32 {\n\
         match h { Held::One(c) => c.value, Held::None => 0 } }",
        "f",
    );
    assert!(
        !ts.contains("c.drop()"),
        "the payload stays the enum\u{2019}s:\n{ts}"
    );
}

/// A `Result` matched against a REFERENCE is READ, not taken apart. `unwrap()`
/// is Rust's `self` form and marks the runtime `Result` moved, so the second
/// read of the same value raised `Result was used after being moved`. RFC 2005
/// again: a pattern matched against a reference binds by reference, and the
/// payload read has to agree with the binding.
#[test]
fn matching_a_borrowed_result_reads_its_payload() {
    const PRELUDE: &str = "pub struct Held { pub n: u32 }\n\
                           pub fn read(h: &Held) -> u32 { h.n }\n";
    let cases = [
        (
            "pub fn f(v: &Result<Held, Held>) -> u32 { \
             match v { Ok(i) => read(i), Err(e) => read(e) } }",
            "f",
        ),
        (
            "pub fn g(v: &Result<Held, Held>) -> u32 { \
             if let Ok(i) = v { read(i) } else { 0 } }",
            "g",
        ),
        (
            "pub fn h(v: &Option<Result<Held, Held>>) -> u32 { \
             match v { Some(Ok(i)) => read(i), Some(Err(e)) => read(e), None => 0 } }",
            "h",
        ),
    ];
    for (rust, method) in cases {
        let mut fixture = crate::testing::Fixture::build(&[(
            "lib.rs",
            &format!("{}{}", PRELUDE, rust),
        )]);
        let ts = fixture.translated_method("lib.rs", method);
        assert!(!ts.contains("unwrap()"), "{method} consumes a borrowed Result:\n{ts}");
        assert!(!ts.contains("unwrapErr()"), "{method} consumes a borrowed Result:\n{ts}");
        assert!(ts.contains("okRef()"), "{method}:\n{ts}");
    }
}

/// An OWNED `Result` is still taken apart: the match is what moved it, and the
/// arm owns what it was handed.
#[test]
fn matching_an_owned_result_still_consumes_it() {
    let mut fixture = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub struct Held { pub n: u32 }\n\
         pub fn read(h: &Held) -> u32 { h.n }\n\
         pub fn f(v: Result<Held, Held>) -> u32 { \
         match v { Ok(i) => read(&i), Err(e) => read(&e) } }",
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert!(ts.contains("v.unwrap()"), "{ts}");
    assert!(ts.contains("v.unwrapErr()"), "{ts}");
    assert!(!ts.contains("okRef"), "{ts}");
}
