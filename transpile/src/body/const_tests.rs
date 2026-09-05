//! What a module-level `const` and a `static` are, and what naming one means.
//!
//! Rust means two different things by the two words. A `const` is a value
//! INLINED at each use, so two uses are two values; a `static` is ONE place for
//! the life of the program, shared on purpose. The port wrote both as one
//! module `const`, which is right for neither: a non-`Copy` const's uses shared
//! an identity, a mutation and a release, and a `static` with an atomic in it
//! could not be written to at all.

use crate::testing::Fixture;

fn emitted(src: &str) -> String {
    let mut fixture = Fixture::build(&[("lib.rs", src)]);
    fixture.emitted("lib.rs")
}

const POINT: &str = "pub struct Point { pub x: u32, pub y: String }\n";

/// #5: a non-`Copy` const is a fresh value at each use, so the emitted name is
/// a function the use calls.
#[test]
fn a_non_copy_const_is_a_fresh_value_at_each_use() {
    let ts = emitted(&format!(
        "{POINT}pub const ORIGIN: Point = Point {{ x: 0, y: String::new() }};\n\
         pub fn read() -> u32 {{ let mut p = ORIGIN; p.x = 9; p.x }}"
    ));
    assert!(
        ts.contains("export function ORIGIN(): Point {"),
        "the const is written as the value it is inlined as:\n{ts}"
    );
    assert!(ts.contains("let p = ORIGIN();"), "and the use calls it:\n{ts}");
}

/// A `Copy` const is a value the assignment already copies, so it stays a
/// binding — a function there would be noise at every use.
#[test]
fn a_copy_const_stays_a_binding() {
    let ts = emitted("pub const LIMIT: u32 = 10;\npub fn read() -> u32 { LIMIT }");
    assert!(ts.contains("export const LIMIT: number = 10;"), "{ts}");
    assert!(ts.contains("return LIMIT;"), "{ts}");
}

/// A `static` of the same non-`Copy` type is ONE place, and shared on purpose.
#[test]
fn a_static_is_one_place_and_stays_a_binding() {
    let ts = emitted(&format!(
        "{POINT}pub static ORIGIN: Point = Point {{ x: 0, y: String::new() }};\n\
         pub fn read() -> u32 {{ ORIGIN.x }}"
    ));
    assert!(ts.contains("export const ORIGIN: Point = new Point("), "{ts}");
    assert!(ts.contains("return ORIGIN.x;"), "{ts}");
}

/// #6: an atomic IS its value here, so a `static` holding one is written to by
/// assigning to the binding — which a `const` binding throws on.
#[test]
fn a_static_with_interior_mutability_is_reassignable() {
    let ts = emitted(
        "use std::sync::atomic::{AtomicUsize, Ordering};\n\
         pub static COUNTER: AtomicUsize = AtomicUsize::new(0);\n\
         pub fn bump() -> usize { COUNTER.fetch_add(1, Ordering::SeqCst) }",
    );
    assert!(ts.contains("export let COUNTER: number = 0;"), "{ts}");
    assert!(ts.contains("COUNTER += 1"), "{ts}");
}

/// F13: `AtomicBool::new` was not lowered at all, so six emitted places built an
/// `AtomicBool` nothing exports — a `ReferenceError` on the line.
#[test]
fn every_mapped_atomic_constructor_is_its_argument() {
    let ts = emitted(
        "use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize};\n\
         pub fn flag() -> AtomicBool { AtomicBool::new(true) }\n\
         pub fn small() -> AtomicU32 { AtomicU32::new(1) }\n\
         pub fn wide() -> AtomicU64 { AtomicU64::new(2) }\n\
         pub fn size() -> AtomicUsize { AtomicUsize::new(3) }",
    );
    assert!(!ts.contains("AtomicBool.new"), "{ts}");
    assert!(!ts.contains("AtomicU32.new"), "{ts}");
    assert!(!ts.contains("AtomicU64.new"), "{ts}");
    assert!(!ts.contains("AtomicUsize.new"), "{ts}");
    assert!(ts.contains("return true;"), "{ts}");
    // The declared types agree with the constructors: every atomic this lowers
    // is one the name map writes as a plain value.
    assert!(ts.contains("function flag(): boolean"), "{ts}");
    for spelled in ["function small(): number", "function wide(): number", "function size(): number"] {
        assert!(ts.contains(spelled), "{spelled}:\n{ts}");
    }
}

/// D4: `-9007199254740991` is one literal in Rust and two tokens here, and the
/// width belongs to the literal. Without it the value was a `number` where a
/// `bigint` was declared — storage-indexeddb's `MIN_SAFE_INTEGER`.
#[test]
fn a_negated_literal_in_a_const_keeps_its_width() {
    let ts = emitted("pub const FLOOR: i64 = -9007199254740991;");
    assert!(ts.contains("= -9007199254740991n;"), "{ts}");
}

/// D12: a name that resolves to a const is a comparison against its value.
/// Read as a binding, `BASE => ..` bound `bASE` and matched everything, and the
/// arms below it were reported as unreachable.
#[test]
fn a_const_in_a_pattern_is_a_comparison() {
    let ts = emitted(
        "pub const BASE: u32 = 36;\n\
         pub fn radix(n: u32) -> u32 { match n { BASE => 1, 0 => 2, _ => 3 } }",
    );
    assert!(ts.contains("if (n === BASE)"), "{ts}");
    assert!(!ts.contains("bASE"), "nothing is bound:\n{ts}");
}

/// A name that resolves to nothing still binds, which is what a pattern's
/// identifier usually is.
#[test]
fn a_name_that_is_not_a_const_still_binds() {
    let ts = emitted("pub fn pick(n: u32) -> u32 { match n { other => other + 1 } }");
    assert!(ts.contains("const other = n;"), "{ts}");
}
