//! Leg A: what the port holds as a `SeqCursor`, asked once.
//!
//! Four decisions read the same predicate — how a signature spells the value,
//! what a call site builds when it hands one over, what the emitter writes on a
//! receiver of that type, and what the scope releases — so each of these cases
//! is a shape where two of them used to disagree.

use crate::testing::Fixture;

const TOKEN: &str = "\
pub struct Token { pub n: i64 }\n\
impl Drop for Token { fn drop(&mut self) {} }\n\
";

fn emitted(rust: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", TOKEN, rust))]);
    let ts = fixture.emitted("lib.rs");
    (ts, fixture.messages())
}

/// AA6: an associated type is a cursor because it IS `IntoIterator`'s
/// `IntoIter` and carries an `Iterator` bound — never because it shares the
/// name. A `type IntoIter: Measure` walks nothing, and `.takeRest()` on one
/// called a method it has not got.
#[test]
fn an_associated_type_that_only_shares_the_name_is_not_a_cursor() {
    let (ts, _) = emitted(
        "pub trait Measure { fn amount(&self) -> i64; }\n\
         pub trait Factory { type IntoIter: Measure; fn make(&self) -> Self::IntoIter; }\n\
         pub fn measured<F>(f: &F) -> i64 where F: Factory { f.make().amount() }",
    );
    assert!(ts.contains("f.make().amount()"), "the call is written as it stands:\n{}", ts);
    assert!(!ts.contains("takeRest"), "and nothing gives up a rest:\n{}", ts);
}

/// Z5: what the SCOPE releases is the same question. An associated
/// `type Out: Iterator` is an iterator the port never wrapped, so its `drop()`
/// names a method it has not got; the honest answer is the report that its glue
/// is unknown.
#[test]
fn an_iterator_the_port_never_wrapped_is_not_released_as_a_cursor() {
    let (ts, messages) = emitted(
        "pub trait Source { type Out: Iterator<Item = Token>; fn open(&self) -> Self::Out; }\n\
         pub fn held<S>(s: &S) -> i64 where S: Source { let it = s.open(); let _ = &it; 7 }",
    );
    assert!(!ts.contains("it.drop()"), "nothing calls a `drop` it has not got:\n{}", ts);
    assert!(
        messages.iter().any(|m| m.contains("cannot say owns anything")),
        "and the gap is reported:\n{:?}",
        messages
    );
}

/// AA2: a concrete caller adapts its SEQUENCE to the cursor the callee walks.
/// Handed the array as it stood, `walk.next()` inside the callee called a
/// method an array has not got.
#[test]
fn a_concrete_sequence_is_wrapped_where_it_crosses_into_a_cursor() {
    let (ts, _) = emitted(
        "pub fn first<I>(mut walk: I) -> Option<Token> where I: Iterator<Item = Token> { walk.next() }\n\
         pub fn concrete(tokens: Vec<Token>) -> Option<Token> { first(tokens.into_iter()) }",
    );
    assert!(
        ts.contains("first(new SeqCursor([...tokens]))"),
        "the sequence is wrapped once, not spread twice:\n{}",
        ts
    );
}

/// And a value that is ALREADY a cursor goes over as it stands.
#[test]
fn a_cursor_handed_to_a_cursor_parameter_is_not_wrapped_again() {
    let (ts, _) = emitted(
        "pub fn first<I>(mut walk: I) -> Option<Token> where I: Iterator<Item = Token> { walk.next() }\n\
         pub fn again<J>(walk: J) -> Option<Token> where J: Iterator<Item = Token> { first(walk) }",
    );
    assert!(ts.contains("first(walk)"), "handed on as it stands:\n{}", ts);
}

/// AA5: a by-value cursor parameter the body never consumes is still the
/// body's, and Rust drops it at the end. Read off the unmodified Rust type it
/// was `Drops::Unknown`, and the cursor and everything left in it leaked.
#[test]
fn a_cursor_parameter_the_body_never_consumes_is_released() {
    let (ts, _) = emitted(
        "pub fn ignore<I>(_walk: I) -> i64 where I: Iterator<Item = Token> { 7 }",
    );
    assert!(ts.contains("_walk.drop()"), "the parameter is released:\n{}", ts);
}

/// AA3: a cursor that gave up its rest is an ARRAY from that point on, and the
/// array's own table is what knows how to write a call on it. Dispatched on the
/// unmodified type parameter, `count` came out `walk.takeRest().count()` — a
/// method no array has — and the array's own table then answers it through the
/// helper that releases what it counted (T6).
#[test]
fn a_cursor_that_gave_up_its_rest_dispatches_as_a_sequence() {
    let (ts, _) = emitted(
        "pub fn counted<I>(walk: I) -> usize where I: Iterator<Item = Token> { walk.count() }",
    );
    assert!(
        ts.contains("countOwned(walk.takeRest())"),
        "`count` is the array's, through the helper that releases what it counted:\n{}",
        ts
    );
}

/// The one method that does NOT: `next` is what the cursor exists for.
#[test]
fn next_is_asked_of_the_cursor_itself() {
    let (ts, _) = emitted(
        "pub fn one<I>(mut walk: I) -> Option<Token> where I: Iterator<Item = Token> { walk.next() }",
    );
    assert!(ts.contains("walk.next()"), "asked of the cursor:\n{}", ts);
    assert!(!ts.contains("takeRest"), "and it keeps its rest:\n{}", ts);
}

/// Z3/AA4: `by_ref()` on a cursor is the IDENTITY. The port has no reference,
/// so the borrowed view IS the cursor — written as a consumption it took the
/// whole rest out and marked the cursor moved, and a `for` loop over it asked
/// the same question a second time.
#[test]
fn by_ref_on_a_cursor_is_the_cursor() {
    let (ts, _) = emitted(
        "pub fn sum_of<I>(mut walk: I) -> i64 where I: Iterator<Item = Token> {\n\
           let mut total = 0i64;\n\
           for token in walk.by_ref() { total += token.n; }\n\
           total\n\
         }",
    );
    assert!(!ts.contains("takeRest"), "the cursor is not consumed:\n{}", ts);
    assert!(!ts.contains("byRef"), "and no method named `byRef` is called:\n{}", ts);
    assert!(ts.contains("walk.drainRest()"), "the walk is drained once:\n{}", ts);
    assert!(ts.contains("walk.drop()"), "and the owner still releases it:\n{}", ts);
}

/// Z4: `&mut I` is an `Iterator` by the blanket impl, so a callee handed the
/// reborrow may run the whole of `Iterator` on it — and Rust leaves the
/// iterator alive for its owner. Consumed there, the owner's own `drop()` was a
/// use after move on a program Rust runs.
#[test]
fn a_reborrowed_cursor_gives_up_its_rest_without_being_consumed() {
    let (ts, _) = emitted(
        "pub fn drain<I>(values: &mut I) -> Vec<Token> where I: Iterator<Item = Token> {\n\
           values.collect()\n\
         }",
    );
    assert!(ts.contains("values.drainRest()"), "drained, not taken:\n{}", ts);
}

/// And a cursor taken BY VALUE still is: every consuming `Iterator` method
/// takes the iterator by value, and the frame that declared it must not release
/// it afterwards.
#[test]
fn a_cursor_by_value_is_consumed_when_it_gives_up_its_rest() {
    let (ts, _) = emitted(
        "pub fn rest<I>(walk: I) -> Vec<Token> where I: Iterator<Item = Token> {\n\
           walk.collect()\n\
         }",
    );
    assert!(ts.contains("walk.takeRest()"), "taken, not drained:\n{}", ts);
    assert!(!ts.contains("drainRest"), "and not both:\n{}", ts);
}

/// The owning shapes ABOVE a borrowed view stay refused: the caller really does
/// keep the rest there, and the port has no borrowed view of a cursor to write.
#[test]
fn an_owning_adaptor_above_a_by_ref_view_is_refused() {
    let (ts, messages) = emitted(
        "pub fn one<I>(mut walk: I) -> Vec<Token> where I: Iterator<Item = Token> {\n\
           walk.by_ref().take(1).collect()\n\
         }",
    );
    assert!(ts.contains("unsupported("), "the shape is a hole:\n{}\n{:?}", ts, messages);
    assert!(
        messages.iter().any(|m| m.contains("leaves the rest in the iterator this receiver names")),
        "and the refusal says why:\n{:?}",
        messages
    );
}

/// The loop over a cursor's rest is the owned-ARRAY loop, because that is what
/// the rest IS. Read as an opaque sequence it carried a report saying the
/// runtime does not write it as an array, and what a `break` left behind was
/// released by nobody.
#[test]
fn a_loop_over_a_cursors_rest_releases_what_it_never_reached() {
    let (ts, messages) = emitted(
        "pub fn walked<I>(walk: I) -> i64 where I: Iterator<Item = Token> {\n\
           let mut total = 0i64;\n\
           for token in walk { total += token.n; if total > 3 { break; } }\n\
           total\n\
         }",
    );
    assert!(ts.contains("dropOwned("), "the tail is released:\n{}", ts);
    assert!(
        !messages.iter().any(|m| m.contains("does not write it as an array")),
        "and nothing says the rest is not an array:\n{:?}",
        messages
    );
}
