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

/// A TUPLE written to be matched is a tuple of scrutinees, and each element
/// carries its own `&`.
///
/// `if let (Ex::Path(p), Ex::Lit(q)) = (&**left, &**right)` matches two
/// BORROWED enums, so neither name is the branch's to release. Reading the
/// tuple's type through `resolve_expr_type` erased both references, and the
/// branch dropped two payloads their owners still hold — the core watcher set's
/// `recurse_predicate_watchers` did exactly this.
#[test]
fn a_borrowed_tuple_subject_releases_neither_binding() {
    let mut fixture = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub enum Ex { Path(Token), Lit(Token) }\n\
         pub fn f(l: &Box<Ex>, r: &Box<Ex>) -> u32 { \
         if let (Ex::Path(p), Ex::Lit(q)) = (&**l, &**r) { p.n + q.n } else { 0 } }",
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert!(!ts.contains("p.drop()"), "the left binding is a borrow:\n{ts}");
    assert!(!ts.contains("q.drop()"), "the right binding is a borrow:\n{ts}");
}

/// An OWNED tuple subject still hands its elements to the branch.
#[test]
fn an_owned_tuple_subject_releases_both_bindings() {
    let mut fixture = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn f(pair: (Token, Token)) -> u32 { \
         if let (a, b) = pair { a.n + b.n } else { 0 } }",
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert!(ts.contains("a.drop()"), "the first element is the branch's:\n{ts}");
    assert!(ts.contains("b.drop()"), "the second element is the branch's:\n{ts}");
}

/// Every alternative of an or-pattern binds the SAME names, so the scope claims
/// each name ONCE.
///
/// Listing the names of every alternative gave `literal` two owners and two
/// releases, and the strict registry aborts on the second — the core watcher
/// set's `Predicate::Comparison` arm is the corpus site.
#[test]
fn an_or_pattern_claims_each_name_once() {
    let mut fixture = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub enum Ex { Path(Token), Lit(Token) }\n\
         pub fn f(e: Ex) -> u32 { \
         if let Ex::Path(t) | Ex::Lit(t) = e { t.n } else { 0 } }",
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert_eq!(ts.matches("t.drop()").count(), 1, "one owner, one release:\n{ts}");
}

/// A `Result` arm owns the payload the side read out, whatever its type turns
/// out to be.
///
/// `T::from_value(v)` behind a type parameter has an error type the engine
/// cannot name, and the arm that bound it wrote `const _v2 = _v1;` and returned
/// without releasing anything — a `PropertyError` left for the collector at
/// four corpus sites. `dropOwned` releases it by its runtime shape.
#[test]
fn a_result_arm_releases_a_payload_the_engine_cannot_name() {
    let mut fixture = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub enum PropertyError { Missing, Bad(String) }\n\
         pub trait Property { \
         fn from_value(value: u32) -> Result<Self, PropertyError> where Self: Sized; }\n\
         pub fn f<T: Property>(value: u32) -> Result<Option<T>, PropertyError> { \
         match T::from_value(value) { \
         Ok(v) => Ok(Some(v)), \
         Err(PropertyError::Missing) => Ok(None), \
         Err(err) => Err(err) } }",
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert!(ts.contains("dropOwned(_v2)"), "the Missing arm owns the error:\n{ts}");
}
