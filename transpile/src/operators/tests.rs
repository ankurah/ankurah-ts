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

/// An overloaded operator is a method call whose impl takes both operands by
/// value: the call releases them, so the block that held them must not.
const ADD: &str = "\
use std::ops::Add;\n\
pub struct Weight { pub label: String, pub grams: u64 }\n\
impl Add for Weight {\n\
  type Output = Weight;\n\
  fn add(self, rhs: Weight) -> Weight { Weight { label: self.label, grams: self.grams + rhs.grams } }\n\
}\n\
";

fn added(rust: &str, method: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", ADD, rust))]);
    let ts = fixture.translated_method("lib.rs", method);
    (ts, fixture.messages())
}

#[test]
fn an_overloaded_operator_takes_both_operands_and_nothing_releases_them_again() {
    let (ts, said) = added(
        "pub fn combined(a: Weight, b: Weight) -> Weight { a + b }",
        "combined",
    );
    assert!(ts.contains("a.add(b)"), "{}", ts);
    assert!(!ts.contains(".drop()"), "{}", ts);
    assert!(said.is_empty(), "{:?}", said);
}

/// The same where a `let` stands between the parameters and the operator: the
/// right operand is a local the type context has not met when the parameters
/// are claimed, and Rust's `Rhs` defaults to `Self`, which is what stands in.
#[test]
fn the_operands_are_taken_even_when_the_right_one_is_a_later_local() {
    let (ts, _) = added(
        "pub fn twice(a: Weight) -> bool {\n\
           let b = Weight { label: String::from(\"x\"), grams: 1 };\n\
           let total = a + b;\n\
           total.grams > 1\n\
         }",
        "twice",
    );
    assert!(!ts.contains("a.drop()"), "{}", ts);
    assert!(!ts.contains("b.drop()"), "{}", ts);
    // What the operator answered is the block's, and the block releases it.
    assert!(ts.contains("total.drop()"), "{}", ts);
}

/// A reference operand is not moved out of, whatever the impl does with the
/// value behind it.
#[test]
fn a_reference_operand_is_left_where_it_was() {
    let (ts, _) = added(
        "pub fn heavier(a: &Weight, b: &Weight) -> bool { a.grams > b.grams }",
        "heavier",
    );
    assert!(!ts.contains(".drop()"), "{}", ts);
}

/// The impl's `Output` is what the operator answers, so the local it is bound
/// to has a type and a release.
#[test]
fn the_result_of_an_overloaded_operator_has_the_impls_output_type() {
    let (ts, said) = added(
        "pub fn label(a: Weight, b: Weight) -> String { let c = a + b; c.label }",
        "label",
    );
    assert!(said.is_empty(), "{:?}", said);
    assert!(ts.contains("const c = a.add(b);"), "{}", ts);
}

/// Rust gives booleans `^`, `&` and `|`; JavaScript reads all three as bit
/// arithmetic on numbers, so `a ^ b` answered `0` or `1`.
#[test]
fn the_bit_operators_on_booleans_are_boolean_operators() {
    let (ts, _) = body("pub fn differ(a: bool, b: bool) -> bool { a ^ b }", "differ");
    assert!(ts.contains("a !== b"), "{}", ts);
    let (ts, _) = body("pub fn both(a: bool, b: bool) -> bool { a & b }", "both");
    assert!(ts.contains("a && b"), "{}", ts);
    let (ts, _) = body("pub fn either(a: bool, b: bool) -> bool { a | b }", "either");
    assert!(ts.contains("a || b"), "{}", ts);
}

/// A right operand that does something of its own is not evaluated by `&&`,
/// and Rust's `&` evaluates it.
#[test]
fn a_side_effecting_right_operand_of_a_boolean_and_is_reported() {
    let (_, said) = body(
        "pub fn check(flag: bool) -> bool { flag & touch() }\n\
         pub fn touch() -> bool { true }",
        "check",
    );
    assert!(
        said.iter().any(|m| m.contains("evaluates both sides")),
        "{:?}",
        said
    );
}

/// JavaScript's bit operators produce a signed 32-bit number whatever they were
/// given, so `>>` on an unsigned type shifted the sign bit in.
#[test]
fn the_bit_operators_on_an_unsigned_integer_stay_unsigned() {
    let (ts, _) = body("pub fn half(n: u32) -> u32 { n >> 1 }", "half");
    assert!(ts.contains("((n >>> 1) >>> 0)"), "{}", ts);
    let (ts, _) = body("pub fn mask(a: u32, b: u32) -> u32 { a & b }", "mask");
    assert!(ts.contains("((a & b) >>> 0)"), "{}", ts);
}

/// A signed type keeps JavaScript's own operator, which is already int32.
#[test]
fn a_signed_shift_keeps_the_arithmetic_one() {
    let (ts, _) = body("pub fn half(n: i32) -> i32 { n >> 1 }", "half");
    assert!(ts.contains("(n >> 1)"), "{}", ts);
    assert!(!ts.contains(">>>"), "{}", ts);
}
