//! Spec 4.4b: the conversion a caller chooses and a callee cannot.

use crate::testing::Fixture;

/// A target with two `From` impls that take the same JavaScript shape, which is
/// why the choice cannot be made at run time.
const SHAPES: &str = "\
pub enum Held { Text(String), Count(i64) }\n\
impl From<String> for Held { fn from(s: String) -> Held { Held::Text(s) } }\n\
impl From<i64> for Held { fn from(n: i64) -> Held { Held::Count(n) } }\n\
pub struct Refused;\n\
";

fn translated(rust: &str, method: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", SHAPES, rust))]);
    let ts = fixture.translated_method("lib.rs", method);
    (ts, fixture.messages())
}

fn whole(rust: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", SHAPES, rust))]);
    let ts = fixture.emitted("lib.rs");
    (ts, fixture.messages())
}

/// The conversion bound the whole mechanism exists for: `wrap` cannot say which
/// `From` impl runs, and the two candidates take the same JavaScript shape, so
/// no runtime test could tell them apart either.
#[test]
fn a_conversion_bound_with_a_concrete_target_grows_a_parameter() {
    let (ts, _) = whole(
        "pub fn wrap<V: TryInto<Held>>(v: V) -> Result<Held, Refused> {\n\
           match v.try_into() { Ok(h) => Ok(h), Err(_) => Err(Refused) }\n\
         }",
    );
    assert!(
        ts.contains("_convV: (value: V) => Result<Held, unknown>"),
        "the conversion arrives as a parameter:\n{}",
        ts
    );
    assert!(ts.contains("_convV(v)"), "and the body reads it:\n{}", ts);
}

/// The `Error = E` a bound writes is what the conversion FAILED with, so it is
/// decided by the conversion rather than by the caller and gets no dictionary.
#[test]
fn the_error_of_a_conversion_bound_gets_no_dictionary_of_its_own() {
    let (ts, _) = whole(
        "pub fn wrap<V, E>(v: V) -> Result<Held, Refused>\n\
         where V: TryInto<Held, Error = E>, E: Into<Refused> {\n\
           let held = v.try_into().map_err(|e| e.into())?;\n\
           Ok(held)\n\
         }",
    );
    assert!(
        ts.contains("_convV: (value: V) => Result<Held, E>"),
        "the parameter answers what `try_into` answers:\n{}",
        ts
    );
    assert!(!ts.contains("_convE"), "and nothing is asked for the error:\n{}", ts);
}

#[test]
fn a_concrete_call_site_writes_the_conversion_it_inferred() {
    let (ts, messages) = translated(
        "pub fn wrap<V: TryInto<Held>>(v: V) -> Result<Held, Refused> {\n\
           match v.try_into() { Ok(h) => Ok(h), Err(_) => Err(Refused) }\n\
         }\n\
         pub fn one() -> Result<Held, Refused> { wrap(7i64) }",
        "one",
    );
    assert!(
        ts.contains("wrap(7n, (value: bigint) => Result.Ok(Held.fromI64(value)))"),
        "the caller writes the impl its own type picks:\n{}\n{:?}",
        ts,
        messages
    );
}

#[test]
fn a_generic_caller_hands_its_own_dictionary_on() {
    let (ts, messages) = translated(
        "pub fn wrap<V: TryInto<Held>>(v: V) -> Result<Held, Refused> {\n\
           match v.try_into() { Ok(h) => Ok(h), Err(_) => Err(Refused) }\n\
         }\n\
         pub fn twice<V: TryInto<Held>>(v: V) -> Result<Held, Refused> { wrap(v) }",
        "twice",
    );
    assert!(
        ts.contains("wrap(v, _convV)"),
        "the caller's own dictionary goes over:\n{}\n{:?}",
        ts,
        messages
    );
}

/// A parameter named only through a bound's item is still a parameter the
/// signature carries a value of, and the caller reads its own bound to say
/// which one of its own it stands for.
#[test]
fn a_parameter_named_through_an_item_binding_is_threaded_too() {
    let (ts, messages) = translated(
        "pub fn walk<I, V>(items: I) -> Result<Held, Refused>\n\
         where I: Iterator<Item = V>, V: TryInto<Held> {\n\
           Err(Refused)\n\
         }\n\
         pub fn outer<J, W>(items: J) -> Result<Held, Refused>\n\
         where J: Iterator<Item = W>, W: TryInto<Held> {\n\
           walk(items)\n\
         }",
        "outer",
    );
    assert!(
        ts.contains("walk(items, _convW)"),
        "the item's own dictionary goes over:\n{}\n{:?}",
        ts,
        messages
    );
}

#[test]
fn a_bound_whose_target_is_open_gets_no_dictionary() {
    let (ts, _) = whole("pub fn wrap<V: TryInto<W>, W>(v: V) -> Option<W> { v.try_into().ok() }");
    assert!(!ts.contains("_convV"), "nothing concrete to convert to:\n{}", ts);
}

/// The scope's limit, and the shape it was measured against: an infallible
/// `Into` bound is how the wasm boundary is written, where the callee is a
/// hand-written file in the port and an extra argument at its call sites would
/// be an argument nothing declares.
#[test]
fn an_infallible_into_bound_keeps_the_diagnostic_it_has() {
    let (ts, _) = whole("pub fn wrap<V: Into<Held>>(v: V) -> Held { v.into() }");
    assert!(!ts.contains("_convV"), "no dictionary for an `Into` bound:\n{}", ts);
}

/// And a target the port writes no class for has nothing to carry.
#[test]
fn a_target_with_no_class_of_its_own_gets_no_dictionary() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        "pub struct Refused;\n\
         pub fn wrap<V: TryInto<u32>>(v: V) -> Option<u32> { v.try_into().ok() }",
    )]);
    let ts = fixture.emitted("lib.rs");
    assert!(!ts.contains("_convV"), "a primitive target carries no impls:\n{}", ts);
}

/// A target whose only conversion from a `String` is a genuine `TryFrom`: the
/// shape `ankql`'s `Selection` has, and the one a `TryInto` bound is written
/// for in the first place.
const FALLIBLE: &str = "\
pub struct Sel;\n\
pub struct Bad;\n\
impl TryFrom<String> for Sel { type Error = Bad; fn try_from(_s: String) -> Result<Sel, Bad> { Ok(Sel) } }\n\
";

/// A concrete site whose conversion IS the fallible one. Its emitted static
/// already answers the `Result` the bound reads, so the call stands unwrapped;
/// asked for a `From` instead, this site found nothing and wrote a hole that
/// threw the moment the callee read the dictionary.
#[test]
fn a_fallible_concrete_site_calls_the_try_from_impl() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub fn pick<V: TryInto<Sel>>(v: V) -> Option<Sel> {{ v.try_into().ok() }}\n\
             pub fn one() -> Option<Sel> {{ pick(String::new()) }}",
            FALLIBLE
        ),
    )]);
    let ts = fixture.translated_method("lib.rs", "one");
    assert!(
        ts.contains("pick('', (value: string) => Sel.tryFrom(value))"),
        "the `TryFrom` impl is called as it stands:\n{}\n{:?}",
        ts,
        fixture.messages()
    );
    assert!(!ts.contains("Result.Ok"), "and its answer is not wrapped twice:\n{}", ts);
    assert!(fixture.messages().is_empty(), "nothing is refused:\n{:?}", fixture.messages());
}

/// The refusal shape, which had no case of its own: a concrete type the impl
/// table converts NEITHER way is a hole, and the arrow still declares what it
/// takes so the hole is not an implicit `any`.
#[test]
fn a_concrete_type_with_no_impl_at_all_is_a_hole() {
    let (ts, messages) = translated(
        "pub fn wrap<V: TryInto<Held>>(v: V) -> Option<Held> { v.try_into().ok() }\n\
         pub fn one() -> Option<Held> { wrap(true) }",
        "one",
    );
    assert!(
        ts.contains("(value: boolean) => unsupported("),
        "the hole carries the type it stands at:\n{}\n{:?}",
        ts,
        messages
    );
    assert!(
        messages.iter().any(|m| m.contains("no impl in the table performs it")),
        "and the refusal says so:\n{:?}",
        messages
    );
}

/// AA1: an argument the engine cannot type holds its place. Dropped instead,
/// every later argument moved one left before the positional match and the
/// call was handed the conversion for a type standing somewhere else — a wrong
/// dictionary at run time, silently.
#[test]
fn an_argument_the_engine_cannot_type_does_not_shift_the_ones_after_it() {
    let (ts, messages) = translated(
        "pub fn pair<A: TryInto<Held>>(a: A, b: i64) -> Option<Held> { let _ = b; a.try_into().ok() }\n\
         pub fn one() -> Option<Held> { pair({ let text = String::new(); text }, 7i64) }",
        "one",
    );
    assert!(
        !ts.contains("Held.fromI64"),
        "the second argument's type does not answer the first parameter:\n{}",
        ts
    );
    assert!(
        ts.contains("unsupported("),
        "the unnameable conversion is a hole:\n{}\n{:?}",
        ts,
        messages
    );
    assert!(
        messages.iter().any(|m| m.contains("the type standing at that parameter is nothing")),
        "and the refusal names the parameter:\n{:?}",
        messages
    );
}

/// The other side of the same rule: a hole LATER in the list leaves the
/// dictionary's own position answering for itself.
#[test]
fn an_untyped_argument_after_the_dictionary_position_changes_nothing() {
    let (ts, messages) = translated(
        "pub fn pair<A: TryInto<Held>>(a: A, b: i64) -> Option<Held> { let _ = b; a.try_into().ok() }\n\
         pub fn one() -> Option<Held> { pair(String::new(), { let n = 7i64; n }) }",
        "one",
    );
    assert!(
        ts.contains("(value: string) => Result.Ok(Held.fromString(value))"),
        "the first argument still picks its own impl:\n{}\n{:?}",
        ts,
        messages
    );
}
