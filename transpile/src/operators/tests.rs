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

/// PREMISE CHANGED (I8): this used to expect `a === b` and a diagnostic saying
/// the JavaScript operator had been written. `===` on two objects compares
/// IDENTITY where Rust compares contents, so what it left was a branch that
/// could never be taken — eight of them were live, `bytes == [0u8; 16]` and two
/// `BTreeSet`s among them. `==` is now performed by the runtime whatever the
/// impl table says, and the site is no longer a gap.
#[test]
fn equality_with_no_impl_is_performed_by_the_runtime() {
    let (ts, messages) = body("pub fn f(a: &Loose, b: &Loose) -> bool { a == b }", "f");
    assert!(ts.contains("valueEquals(a, b)"), "{}", ts);
    assert!(!ts.contains("a === b"), "{}", ts);
    assert!(
        !messages.iter().any(|m| m.contains("compares references rather than values")),
        "the comparison is written, so nothing is reported: {:?}",
        messages
    );
}

/// The negation is written out rather than `!valueEquals(..)`, so the emitted
/// text reads as one call and no parenthesisation question arises around it.
#[test]
fn inequality_with_no_impl_is_the_negated_runtime_comparison() {
    let (ts, _) = body("pub fn f(a: &Loose, b: &Loose) -> bool { a != b }", "f");
    assert!(ts.contains("valueNotEquals(a, b)"), "{}", ts);
}

/// Two sequences are compared element by element — `===` on two arrays is
/// identity, so `bytes == [0u8; 16]` was ALWAYS false.
#[test]
fn two_byte_buffers_are_compared_by_content() {
    let (ts, _) = body("pub fn f(a: &Vec<u8>, b: &Vec<u8>) -> bool { a == b }", "f");
    assert!(ts.contains("valueEquals(a, b)"), "{}", ts);
}

/// An ORDERING with no impl is untouched: `<` between two objects is not a
/// question the runtime can answer without a `compareTo`, and the diagnostic
/// there still stands.
#[test]
fn an_ordering_with_no_impl_still_reports() {
    let (ts, messages) = body("pub fn f(a: &Loose, b: &Loose) -> bool { a < b }", "f");
    assert!(ts.contains("a < b"), "{}", ts);
    assert!(
        messages.iter().any(|m| m.contains("no impl in the table performs it")),
        "{:?}",
        messages
    );
}

/// PREMISE CHANGED (R7): integer division goes through `checkedDiv`, which
/// truncates towards zero as `Math.trunc` did AND panics on a zero divisor as
/// Rust does. `Math.trunc(n / 0)` is `Infinity`.
#[test]
fn integer_division_truncates_and_refuses_a_zero_divisor() {
    let (ts, _) = body("pub fn f(n: u32) -> u32 { n / 2 }", "f");
    assert!(ts.contains("checkedDiv(n, 2, 'u32')"), "{}", ts);
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
///
/// PREMISE CHANGED (R7): the addition itself is `checkedAdd`, which panics on
/// overflow as the debug build does. What this test is about — the literal's
/// width — is unchanged and is what the `1n` says.
#[test]
fn a_literal_written_against_a_64_bit_operand_is_a_bigint() {
    let (ts, messages) = body("pub fn f(n: u64) -> u64 { n + 1 }", "f");
    assert!(ts.contains("checkedAdd(n, 1n, 'u64')"), "{}\n{:?}", ts, messages);
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
///
/// PREMISE CHANGED 2026-09-04: `&` and `|` used to be written as `&&` and `||`,
/// which agree in value and not in what runs — `&&` skips its right operand
/// once the left has decided, and Rust's `&` never does. They are calls now, so
/// both operands are evaluated, left to right, exactly once.
#[test]
fn the_bit_operators_on_booleans_are_boolean_operators() {
    let (ts, _) = body("pub fn differ(a: bool, b: bool) -> bool { a ^ b }", "differ");
    assert!(ts.contains("a !== b"), "{}", ts);
    let (ts, _) = body("pub fn both(a: bool, b: bool) -> bool { a & b }", "both");
    assert!(ts.contains("boolAnd(a, b)"), "{}", ts);
    let (ts, _) = body("pub fn either(a: bool, b: bool) -> bool { a | b }", "either");
    assert!(ts.contains("boolOr(a, b)"), "{}", ts);
}

/// PREMISE CHANGED 2026-09-04: a right operand that does something of its own
/// used to be reported, because `&&` would not have evaluated it. The eager
/// form evaluates it, so there is nothing left to report — and the test that
/// asked for the report now asks for the call.
#[test]
fn a_side_effecting_right_operand_of_a_boolean_and_is_evaluated() {
    let (ts, said) = body(
        "pub fn check(flag: bool) -> bool { flag & touch() }\n\
         pub fn touch() -> bool { true }",
        "check",
    );
    assert!(ts.contains("boolAnd(flag, touch())"), "{}", ts);
    assert!(
        !said.iter().any(|m| m.contains("evaluates both sides")),
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

/// An impl written for REFERENCES is an impl of its own. Rust does not search
/// operator impls through a reference or through `Deref` — `W + N` with only
/// `impl Add<N> for N` and `W: Deref<Target = N>` is E0369 — so
/// `impl Add<&R> for &L` has to be found exactly, and the method it names has
/// to exist. The old test with this name compared two primitive fields and
/// never invoked an overloaded reference operator at all.
#[test]
fn an_operator_impl_written_for_references_is_found_and_called() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::ops::Add;\n\
         pub struct L { pub n: u32 }\n\
         pub struct R { pub n: u32 }\n\
         impl Add<&R> for &L {\n\
           type Output = u32;\n\
           fn add(self, rhs: &R) -> u32 { self.n + rhs.n }\n\
         }\n\
         pub fn refs(a: &L, b: &R) -> u32 { a + b }",
    )]);
    let ts = f.translated_method("lib.rs", "refs");
    assert!(ts.contains("a.add(b)"), "{}", ts);
    assert!(f.messages().is_empty(), "{:?}", f.messages());
    // And the method the call names is on the class the reference points at.
    let emitted = f.emitted("lib.rs");
    assert!(emitted.contains("add(rhs: R): number"), "{}", emitted);
}

/// A heterogeneous `Rhs` resolved only as a LATER local still moves the left
/// operand: the impl consumes it, so the block must not release it as well.
#[test]
fn a_heterogeneous_operator_moves_its_left_operand_even_from_a_later_local() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::ops::Add;\n\
         pub struct Left { pub n: u32 }\n\
         pub struct Right { pub n: u32 }\n\
         impl Add<Right> for Left {\n\
           type Output = u32;\n\
           fn add(self, rhs: Right) -> u32 { self.n + rhs.n }\n\
         }\n\
         pub fn local(left: Left) -> u32 {\n\
           let right = Right { n: 2 };\n\
           left + right\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "local");
    assert!(ts.contains("left.add(right)"), "{}", ts);
    assert!(!ts.contains("left.drop()"), "{}", ts);
}

/// A generic impl says what it answers in terms of its own parameters, and the
/// match that selected it says which ones this site has. Refusing every generic
/// impl left the result local untyped, so nothing released it.
#[test]
fn a_generic_operator_impls_output_is_substituted() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::ops::Add;\n\
         pub struct Generic<T> { pub value: T }\n\
         impl<T> Add for Generic<T> {\n\
           type Output = Generic<T>;\n\
           fn add(self, rhs: Generic<T>) -> Generic<T> { rhs }\n\
         }\n\
         pub fn generic(left: Generic<u32>, right: Generic<u32>) -> u32 {\n\
           let result = left + right;\n\
           result.value\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "generic");
    assert!(ts.contains("const result = left.add(right);"), "{}", ts);
    assert!(ts.contains("result.drop()"), "{}", ts);
}

/// Two written impls whose methods land on one class member: the second used to
/// be dropped without a word, and every call to it went to the first.
#[test]
fn two_operator_impls_that_emit_one_method_name_are_reported() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::ops::Add;\n\
         pub struct W { pub n: u32 }\n\
         pub struct R { pub n: u32 }\n\
         impl Add for W {\n\
           type Output = u32;\n\
           fn add(self, rhs: W) -> u32 { self.n + rhs.n }\n\
         }\n\
         impl Add<R> for W {\n\
           type Output = u32;\n\
           fn add(self, rhs: R) -> u32 { self.n + rhs.n }\n\
         }",
    )]);
    let _ = f.emitted("lib.rs");
    assert!(
        f.messages().iter().any(|m| m.contains("already has that name")),
        "{:?}",
        f.messages()
    );
}

/// A shift by a literal at or past the left operand's width is proof the type
/// is a guess and the guess is wrong: Rust rejects `1u32 << 63` outright.
#[test]
fn a_shift_past_the_guessed_width_is_reported_rather_than_wrapped() {
    let (ts, said) = body("pub fn wide(n: u32) -> u32 { n << 63 }", "wide");
    assert!(said.iter().any(|m| m.contains("shifts by 63")), "{:?}", said);
    assert!(!ts.contains(">>> 0"), "{}", ts);
}

/// `-a`, `!a` and `a[i]` on anything but a primitive are method calls in Rust,
/// and the port wrote the JavaScript operator: `-object` is `NaN` and
/// `object[0]` is `undefined`, neither with a word said.
#[test]
fn the_unary_operators_and_indexing_resolve_through_their_impls() {
    let source = "use std::ops::{Neg, Not, Index};\n\
                  pub struct S { pub n: i32 }\n\
                  impl Neg for S { type Output = S; fn neg(self) -> S { S { n: -self.n } } }\n\
                  impl Not for S { type Output = S; fn not(self) -> S { S { n: !self.n } } }\n\
                  impl Index<usize> for S { type Output = i32; fn index(&self, i: usize) -> &i32 { &self.n } }\n";
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        &format!("{}pub fn negated(a: S) -> S {{ -a }}", source),
    )]);
    let ts = f.translated_method("lib.rs", "negated");
    assert!(ts.contains("a.neg()"), "{}", ts);
    // `Neg::neg` takes self by value, so the block must not release it again.
    assert!(!ts.contains("a.drop()"), "{}", ts);

    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        &format!("{}pub fn notted(a: S) -> S {{ !a }}", source),
    )]);
    assert!(f.translated_method("lib.rs", "notted").contains("a.not()"));

    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        &format!("{}pub fn at(a: &S) -> i32 {{ a[0] }}", source),
    )]);
    assert!(f.translated_method("lib.rs", "at").contains("a.index(0)"));
}

/// And an operand with no impl says so, where it used to say nothing.
#[test]
fn a_unary_operator_with_no_impl_is_reported() {
    let (_, said) = body("pub struct T { pub n: i32 }\npub fn negated(a: T) -> T { -a }", "negated");
    assert!(said.iter().any(|m| m.contains("resolves through `Neg`")), "{:?}", said);
}

/// R7: `+`, `-` and `*` on a fixed-width integer PANIC on overflow, as the
/// `debug_assertions = true` build this port mirrors does. JavaScript wraps
/// nothing and saturates nothing — it goes on counting in doubles, silently
/// losing precision above 2^53 — so a bare `a + b` was a third answer, neither
/// Rust's release wrap nor Rust's debug panic.
#[test]
fn arithmetic_on_an_integer_goes_through_the_checked_helper() {
    let (ts, _) = body("pub fn f(a: u8, b: u8) -> u8 { a + b }", "f");
    assert!(ts.contains("checkedAdd(a, b, 'u8')"), "{}", ts);
    let (ts, _) = body("pub fn f(a: i32, b: i32) -> i32 { a - b }", "f");
    assert!(ts.contains("checkedSub(a, b, 'i32')"), "{}", ts);
    let (ts, _) = body("pub fn f(a: u64, b: u64) -> u64 { a * b }", "f");
    assert!(ts.contains("checkedMul(a, b, 'u64')"), "{}", ts);
}

/// Floats are untouched: Rust's `f64` arithmetic is IEEE and so is
/// JavaScript's.
#[test]
fn float_arithmetic_is_the_javascript_one() {
    let (ts, _) = body("pub fn f(a: f64, b: f64) -> f64 { a + b }", "f");
    assert!(ts.contains("a + b"), "{}", ts);
    assert!(!ts.contains("checked"), "{}", ts);
}

/// The helper is skipped only where the ANSWER is provably in range, not where
/// the OPERANDS are: `255 + 1` on a `u8` has two operands that fit and an
/// answer that does not, and Rust panics on it.
#[test]
fn the_helper_is_skipped_only_when_the_answer_fits() {
    let (ts, _) = body("pub fn f() -> u8 { 1 + 2 }", "f");
    assert!(!ts.contains("checkedAdd"), "two small literals need no check:\n{}", ts);
    let (ts, _) = body("pub fn f() -> u8 { 255 + 1 }", "f");
    assert!(ts.contains("checkedAdd"), "the ANSWER is what overflows:\n{}", ts);
}

/// C1: a `&mut T` parameter whose `T` the port writes as a JavaScript VALUE is
/// a `BorrowMut<T>`. JavaScript passes a number, a string and a boolean by
/// value, so a plain parameter carried the callee's writes nowhere: every axis
/// of ankql's `selection/sql.ts` answered the empty string.
#[test]
fn a_mut_reference_to_a_value_is_a_cell() {
    let (ts, _) = body(
        "fn fill(buffer: &mut String, found: &mut usize) { buffer.push_str(\"?\"); *found += 1; }",
        "fill",
    );
    assert!(ts.contains("buffer.value += '?'"), "{}", ts);
    // R7 reaches through the cell: `*found += 1` is `usize` arithmetic, and a
    // plain `+=` on the cell skipped the overflow check the same statement gets
    // when the place is a local.
    assert!(
        ts.contains("found.value = checkedAdd(found.value, 1, 'usize')"),
        "{}",
        ts
    );
}

/// A `&mut` to a class is already a reference in JavaScript and needs no cell.
#[test]
fn a_mut_reference_to_an_object_needs_no_cell() {
    let (ts, _) = body(
        "pub struct Row { pub n: u32 }\nfn bump(row: &mut Row) { row.n = 1; }",
        "bump",
    );
    assert!(ts.contains("row.n = 1"), "{}", ts);
    assert!(!ts.contains(".value"), "{}", ts);
}

/// K8: `-` on a resolved SIGNED width goes through the runtime's `checkedNeg`.
///
/// A signed width's `MIN` has no positive of its own, and Rust's debug build
/// panics there. JavaScript's `-` answered `2147483648` for an `i32` and said
/// nothing; `abs()` has gone through the same helper since Z8.
#[test]
fn negating_a_signed_integer_goes_through_the_checked_helper() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub fn negate(n: i32) -> i32 { -n }\n\
         pub fn wide(n: i64) -> i64 { -n }\n\
         pub fn float(x: f64) -> f64 { -x }\n\
         pub fn smallest() -> i32 { -2147483648 }\n\
         pub fn unsigned_diff(a: u32, b: u32) -> u32 { a - b }",
    )]);
    assert!(f.translated_method("lib.rs", "negate").contains("checkedNeg(n, 'i32')"));
    assert!(f.translated_method("lib.rs", "wide").contains("checkedNeg(n, 'i64')"));
    // A float keeps the operator: IEEE negation is total.
    let float = f.translated_method("lib.rs", "float");
    assert!(float.contains("-x"), "{}", float);
    assert!(!float.contains("checkedNeg"), "{}", float);
    // And a literal keeps it: `-2147483648` is how `i32::MIN` is written, and
    // the helper would raise on the literal it is written from.
    let smallest = f.translated_method("lib.rs", "smallest");
    assert!(smallest.contains("-2147483648"), "{}", smallest);
    assert!(!smallest.contains("checkedNeg"), "{}", smallest);
}
