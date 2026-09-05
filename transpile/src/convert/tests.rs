//! What the emitted TypeScript does at a conversion, rule by rule.

use crate::testing::Fixture;

/// Two error types with one `From` between them, so a `?` across them has an
/// impl to find, and a third with none, so a `?` across those has not.
const ERRORS: &str = "\
pub struct Wire;\n\
pub struct Wrapped;\n\
pub struct Stray;\n\
impl From<Wire> for Wrapped { fn from(e: Wire) -> Wrapped { Wrapped } }\n\
pub fn g() -> Result<u32, Wire> { Ok(1) }\n\
pub fn stray() -> Result<u32, Stray> { Ok(1) }\n\
";

fn translated(rust: &str, method: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", ERRORS, rust))]);
    let ts = fixture.translated_method("lib.rs", method);
    (ts, fixture.messages())
}

#[test]
fn a_question_mark_across_two_error_types_calls_the_from_impl() {
    let (ts, messages) = translated(
        "pub fn f() -> Result<u32, Wrapped> { let n = g()?; Ok(n) }",
        "f",
    );
    assert!(
        ts.contains("Result.Err(Wrapped.fromWire(_r0.unwrapErr()))"),
        "{}\n{:?}",
        ts,
        messages
    );
    assert!(
        !messages.iter().any(|m| m.contains("`?` converts")),
        "{:?}",
        messages
    );
}

#[test]
fn a_question_mark_between_one_error_type_writes_no_conversion() {
    let (ts, _) = translated(
        "pub fn f() -> Result<u32, Wire> { let n = g()?; Ok(n) }",
        "f",
    );
    assert!(ts.contains("Result.Err(_r0.unwrapErr())"), "{}", ts);
}

#[test]
fn a_question_mark_with_no_from_impl_says_so_and_hands_the_error_on() {
    let (ts, messages) = translated(
        "pub fn f() -> Result<u32, Wrapped> { let n = stray()?; Ok(n) }",
        "f",
    );
    assert!(ts.contains("Result.Err(_r0.unwrapErr())"), "{}", ts);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("no impl in the table performs it")),
        "{:?}",
        messages
    );
}

/// The reflexive `impl<T> From<T> for T` is written for every type in the
/// language. Matching the target and the source in separate substitutions would
/// let it stand for a conversion between two different types, and every `?`
/// would resolve to a conversion that does nothing.
#[test]
fn the_reflexive_from_impl_does_not_convert_two_different_types() {
    let (_, messages) = translated(
        "pub fn f() -> Result<u32, Wrapped> { let n = stray()?; Ok(n) }",
        "f",
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("no impl in the table performs it")),
        "the reflexive impl answered a conversion between two types: {:?}",
        messages
    );
}

/// Rust allows `?` on an `Option` only in a function that returns one, so a
/// `Result` return here means the engine read one of the two types wrongly.
#[test]
fn an_option_question_mark_in_a_result_function_is_reported() {
    let (_, messages) = translated(
        "pub fn m() -> Option<u32> { None }\n\
         pub fn f() -> Result<u32, Wire> { let n = m()?; Ok(n) }",
        "f",
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("inside a function returning a `Result`")),
        "{:?}",
        messages
    );
}

/// A crate whose conversions cover the three answers: an impl the corpus wrote,
/// a conversion the runtime performs, and one nothing performs.
const CONVERSIONS: &str = "\
pub struct Tag { pub id: u32 }\n\
pub struct Name { pub text: String }\n\
impl From<Tag> for Name { fn from(t: Tag) -> Name { Name { text: String::new() } } }\n\
pub struct Loose;\n\
";

fn converted(rust: &str, method: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", CONVERSIONS, rust))]);
    let ts = fixture.translated_method("lib.rs", method);
    (ts, fixture.messages())
}

#[test]
fn into_calls_the_impl_the_position_names() {
    let (ts, messages) = converted("pub fn f(t: Tag) -> Name { t.into() }", "f");
    assert!(ts.contains("Name.fromTag(t)"), "{}\n{:?}", ts, messages);
}

#[test]
fn a_target_qualified_from_calls_the_same_impl() {
    let (ts, _) = converted("pub fn f(t: Tag) -> Name { Name::from(t) }", "f");
    assert!(ts.contains("Name.fromTag(t)"), "{}", ts);
}

#[test]
fn into_with_no_expected_type_says_so_and_writes_the_value() {
    let (ts, messages) = converted(
        "pub fn g(n: Name) -> u32 { 1 }\npub fn f(t: Tag) -> u32 { let x = t.into(); g(x) }",
        "f",
    );
    assert!(!ts.contains(".into()"), "{}", ts);
    assert!(
        messages.iter().any(|m| m.contains("has no expected type here")),
        "{:?}",
        messages
    );
}

#[test]
fn to_string_on_a_string_is_the_string() {
    let (ts, _) = converted("pub fn f(s: &str) -> String { s.to_string() }", "f");
    assert!(!ts.contains("toString"), "{}", ts);
    assert!(ts.contains("return s;"), "{}", ts);
}

#[test]
fn a_widening_cast_into_a_64_bit_integer_crosses_into_bigint() {
    let (ts, _) = converted("pub fn f(n: u32) -> u64 { n as u64 }", "f");
    assert!(ts.contains("BigInt(n)"), "{}", ts);
}

#[test]
fn a_narrowing_cast_keeps_the_low_bits() {
    let (ts, _) = converted("pub fn f(n: u64) -> u32 { n as u32 }", "f");
    assert!(ts.contains("Number(BigInt.asUintN(32, n))"), "{}", ts);
}

#[test]
fn a_cast_that_is_not_between_two_numbers_is_reported() {
    let (_, messages) = converted("pub fn f(t: &Tag) -> u32 { t.id }", "f");
    assert!(messages.is_empty(), "{:?}", messages);
    let (ts, messages) = converted("pub fn f(n: u32) -> u32 { n as u32 }", "f");
    assert!(ts.contains("return n;"), "{}\n{:?}", ts, messages);
}

#[test]
fn a_conversion_no_impl_performs_says_so() {
    let (_, messages) = converted("pub fn f(l: Loose) -> Name { l.into() }", "f");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("no impl in the table performs it")),
        "{:?}",
        messages
    );
}

// ── Casts, where the two languages answer differently ────────────────

fn cast_body(rust: &str, method: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", rust)]);
    let ts = fixture.translated_method("lib.rs", method);
    (ts, fixture.messages())
}

/// `u64 as f64` used to be unreachable: the width lookup ran first and has no
/// answer for a float, so the cast was reported and the bigint handed on where
/// a `number` was wanted.
#[test]
fn a_bigint_widened_to_a_float_is_a_number() {
    let (ts, said) = cast_body("pub fn wide(n: u64) -> f64 { n as f64 }", "wide");
    assert!(ts.contains("Number(n)"), "{}", ts);
    assert!(said.is_empty(), "{:?}", said);
}

/// Rust's float-to-integer `as` saturates and answers 0 for NaN. It is the one
/// `as` that does not wrap.
#[test]
fn a_float_narrowed_to_an_integer_saturates() {
    let (ts, said) = cast_body("pub fn narrow(f: f64) -> u32 { f as u32 }", "narrow");
    assert!(
        ts.contains("Math.min(Math.max(Math.trunc(f) || 0, 0), 4294967295)"),
        "{}",
        ts
    );
    assert!(said.is_empty(), "{:?}", said);
}

/// The port writes a `char` as a one-character string, and Rust's casts through
/// it are about its code point.
#[test]
fn a_char_casts_through_its_code_point() {
    let (ts, said) = cast_body("pub fn code(c: char) -> u32 { c as u32 }", "code");
    assert!(ts.contains("c.codePointAt(0) ?? 0"), "{}", ts);
    assert!(said.is_empty(), "{:?}", said);
    let (ts, said) = cast_body("pub fn letter(b: u8) -> char { b as char }", "letter");
    assert!(ts.contains("String.fromCharCode(Number(b))"), "{}", ts);
    assert!(said.is_empty(), "{:?}", said);
}

/// `to_owned` on a number is the number: there is nothing to clone, and
/// `n.clone()` was a TypeError.
#[test]
fn to_owned_on_a_primitive_is_the_value() {
    let (ts, _) = cast_body("pub fn own(n: &u32) -> u32 { n.to_owned() }", "own");
    assert!(!ts.contains(".clone()"), "{}", ts);
    assert!(ts.contains("return n;"), "{}", ts);
}

/// `Target::from(x)` goes through the same three questions `.into()` does: is
/// it the identity, is it a conversion the runtime performs, and only then is
/// it an impl. Asking the impl table alone found the surface's
/// `impl From<&str> for String`, failed to write a call for it, and fell
/// through to `String.from(s)`, which is not a function.
#[test]
fn a_qualified_conversion_goes_through_the_native_table_first() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub fn text(s: &str) -> String { String::from(s) }",
    )]);
    assert_eq!(f.translated_method("lib.rs", "text").trim(), "return s;");

    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Held { pub n: u32 }\n\
         pub fn shared(h: Held) -> Arc<Held> { Arc::from(h) }",
    )]);
    assert!(f.translated_method("lib.rs", "shared").contains("Arc.new(h)"));
}

/// And a conversion nothing performs says so, where it used to write a call to
/// a static that is not there.
#[test]
fn a_qualified_conversion_with_no_impl_is_reported() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub struct A { pub n: u32 }\n\
         pub struct B { pub n: u32 }\n\
         pub fn convert(a: A) -> B { B::from(a) }",
    )]);
    let _ = f.translated_method("lib.rs", "convert");
    assert!(
        f.messages().iter().any(|m| m.contains("no impl converts them")),
        "{:?}",
        f.messages()
    );
}

/// The port emits a type alias under its own name, and a resolved type has no
/// memory of the alias it was written as. Reading only the OUTERMOST name left
/// `&Listener`, `Vec<Listener>` and a struct field written as `Listener`
/// expanded into the `Arc<dyn Fn(T)>` the alias stands for.
#[test]
fn an_alias_survives_a_reference_a_wrapper_and_a_field() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub type Listener = Arc<dyn Fn(u32) + Send + Sync>;\n\
         pub struct Holder { pub one: Listener, pub many: Vec<Listener> }\n\
         pub fn take(l: &Listener, all: &[Listener]) -> Vec<Listener> { all.to_vec() }",
    )]);
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("readonly one: Listener;"), "{}", ts);
    assert!(ts.contains("readonly many: Listener[];"), "{}", ts);
    assert!(ts.contains("take(l: Listener, all: Listener[]): Listener[]"), "{}", ts);
}
