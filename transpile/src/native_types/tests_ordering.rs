//! `std::cmp::Ordering` as the number a comparison answers.

use crate::testing::Fixture;

fn body(src: &str, method: &str) -> String {
    let mut f = Fixture::build(&[("lib.rs", &format!("use std::cmp::Ordering;\n{}", src))]);
    f.translated_method("lib.rs", method)
}

/// An explicit `.cmp(..)` used to keep its Rust name and reach nothing: no
/// emitted class declares `cmp`, and the derive writes `compareTo`.
#[test]
fn an_explicit_cmp_call_is_the_ordering_method_the_derive_writes() {
    let ts = body(
        "#[derive(PartialEq, Eq, PartialOrd, Ord)]\n\
         pub struct Key { pub n: u32 }\n\
         pub fn order(a: &Key, b: &Key) -> Ordering { a.cmp(b) }",
        "order",
    );
    assert!(ts.contains("a.compareTo(b)"), "{}", ts);
}

/// `Ordering::Greater` had no spelling at all: it came out as
/// `undefined /* Ordering */.Greater`, a garbage expression compared against a
/// number.
#[test]
fn the_three_orderings_are_the_numbers_a_comparison_answers() {
    let ts = body(
        "#[derive(PartialEq, Eq, PartialOrd, Ord)]\n\
         pub struct Key { pub n: u32 }\n\
         pub fn bigger(a: &Key, b: &Key) -> bool { a.cmp(b) == Ordering::Greater }",
        "bigger",
    );
    assert!(ts.contains("=== 1"), "{}", ts);
}

/// Everything `Ordering` declares is one expression on a number.
#[test]
fn orderings_own_methods_are_written_out() {
    let prelude = "#[derive(PartialEq, Eq, PartialOrd, Ord)]\n\
                   pub struct Key { pub n: u32, pub tag: String }\n";
    let ts = body(
        &format!("{}pub fn tie(a: &Key, b: &Key) -> Ordering {{ a.n.cmp(&b.n).then_with(|| a.tag.cmp(&b.tag)) }}", prelude),
        "tie",
    );
    assert!(ts.contains("$c !== 0 ? $c :"), "{}", ts);
    let ts = body(&format!("{}pub fn back(a: &Key, b: &Key) -> Ordering {{ a.cmp(b).reverse() }}", prelude), "back");
    assert!(ts.contains("-(a.compareTo(b))"), "{}", ts);
    let ts = body(&format!("{}pub fn under(a: &Key, b: &Key) -> bool {{ a.cmp(b).is_lt() }}", prelude), "under");
    assert!(ts.contains("a.compareTo(b) < 0"), "{}", ts);
}

/// A primitive has no `compareTo` for the call to land on, so the comparison is
/// written out — once per side, whatever expression each side is.
#[test]
fn cmp_on_a_primitive_is_written_out() {
    let ts = body(
        "pub fn order(a: u32, b: u32) -> Ordering { a.cmp(&b) }",
        "order",
    );
    assert!(ts.contains("$a < $b ? -1 : $a > $b ? 1 : 0"), "{}", ts);
}

/// `Vec::sort()` orders by `Ord`; JavaScript's argument-less `sort` orders by
/// `String(value)`, which for a `Vec<Key>` is `[object Object]` for every
/// element.
#[test]
fn an_argument_less_sort_orders_by_the_types_own_comparison() {
    let ts = body(
        "#[derive(PartialEq, Eq, PartialOrd, Ord)]\n\
         pub struct Key { pub n: u32 }\n\
         pub fn sorted(mut v: Vec<Key>) -> Vec<Key> { v.sort(); v }",
        "sorted",
    );
    assert!(ts.contains("v.sort((a, b) => a.compareTo(b))"), "{}", ts);
}
