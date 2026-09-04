//! What each operator is written as, once the operands are named.

use crate::testing::Fixture;

/// One type that derives `PartialEq` — so `==` on it is the `equals` the derive
/// emitted — and one that does not.
const TYPES: &str = "\
#[derive(PartialEq)]\n\
pub struct Tag { pub id: u32 }\n\
pub struct Loose { pub id: u32 }\n\
";

fn body(rust: &str, method: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", TYPES, rust))]);
    let ts = fixture.translated_method("lib.rs", method);
    (ts, fixture.messages())
}

#[test]
fn equality_between_two_objects_is_the_types_own_equals() {
    let (ts, messages) = body("pub fn f(a: &Tag, b: &Tag) -> bool { a == b }", "f");
    assert!(ts.contains("a.equals(b)"), "{}\n{:?}", ts, messages);
}

#[test]
fn inequality_negates_the_same_call() {
    let (ts, _) = body("pub fn f(a: &Tag, b: &Tag) -> bool { a != b }", "f");
    assert!(ts.contains("!a.equals(b)"), "{}", ts);
}

#[test]
fn equality_with_no_impl_says_so_and_keeps_the_operator() {
    let (ts, messages) = body("pub fn f(a: &Loose, b: &Loose) -> bool { a == b }", "f");
    assert!(ts.contains("a === b"), "{}", ts);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("no impl in the table performs it")),
        "{:?}",
        messages
    );
}

#[test]
fn integer_division_truncates() {
    let (ts, _) = body("pub fn f(n: u32) -> u32 { n / 2 }", "f");
    assert!(ts.contains("Math.trunc(n / 2)"), "{}", ts);
}

#[test]
fn float_division_does_not() {
    let (ts, _) = body("pub fn f(n: f64) -> f64 { n / 2.0 }", "f");
    assert!(!ts.contains("Math.trunc"), "{}", ts);
}

#[test]
fn the_complement_of_an_integer_flips_bits_and_stays_in_range() {
    let (ts, _) = body("pub fn f(n: u32) -> u32 { !n }", "f");
    assert!(ts.contains("(~n >>> 0)"), "{}", ts);
}

#[test]
fn the_negation_of_a_boolean_is_the_javascript_one() {
    let (ts, _) = body("pub fn f(b: bool) -> bool { !b }", "f");
    assert!(ts.contains("!b"), "{}", ts);
    assert!(!ts.contains('~'), "{}", ts);
}

/// A `bigint` beside a `number` throws in JavaScript rather than adding, so a
/// literal written against a 64-bit operand has to be a `bigint` too.
#[test]
fn a_literal_written_against_a_64_bit_operand_is_a_bigint() {
    let (ts, messages) = body("pub fn f(n: u64) -> u64 { n + 1 }", "f");
    assert!(ts.contains("n + 1n"), "{}\n{:?}", ts, messages);
}

#[test]
fn a_shift_amount_beside_a_bigint_is_converted() {
    let (ts, _) = body("pub fn f(n: u64, by: u32) -> u64 { n << by }", "f");
    assert!(ts.contains("n << BigInt(by)"), "{}", ts);
}
