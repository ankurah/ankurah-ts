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

// ── FF1: the REST axis — which methods consume the cursor ──

/// FF1: `any` takes the iterator by `&mut self` and stops at the first match,
/// so the cursor is neither consumed nor emptied — it is asked. Written through
/// the rest, `walk.takeRest().some(p)` marked the cursor moved with the owner's
/// own `walk.drop()` still below it (a use after move) and left every element
/// after the match owned by nobody.
#[test]
fn a_short_circuiting_by_reference_method_is_asked_of_the_cursor_itself() {
    let (ts, _) = emitted(
        "pub fn seen<I>(mut walk: I) -> bool where I: Iterator<Item = Token> \
         { walk.any(|t| t.n == 1) }",
    );
    assert!(ts.contains("walk.any("), "the cursor answers it:\n{}", ts);
    assert!(!ts.contains("takeRest"), "nothing consumes the cursor:\n{}", ts);
    assert!(!ts.contains("drainRest"), "and nothing empties it:\n{}", ts);
}

/// The same for the other five short-circuiting names and for `size_hint`,
/// which takes `&self` and reads.
#[test]
fn every_in_place_cursor_method_is_written_on_the_cursor() {
    for (rust, ts_name) in [
        ("walk.all(|t| t.n == 1)", "walk.all("),
        ("walk.find(|t| t.n == 1).is_some()", "walk.find("),
        ("walk.position(|t| t.n == 1).is_some()", "walk.position("),
        ("walk.nth(2).is_some()", "walk.nth("),
    ] {
        let (ts, _) = emitted(&format!(
            "pub fn asked<I>(mut walk: I) -> bool where I: Iterator<Item = Token> {{ {} }}",
            rust
        ));
        assert!(ts.contains(ts_name), "`{}` is asked of the cursor:\n{}", rust, ts);
        assert!(!ts.contains("takeRest"), "`{}` consumes nothing:\n{}", rust, ts);
    }
}

/// And a method that DOES take the iterator by value still consumes it.
#[test]
fn a_by_value_terminal_still_takes_the_rest() {
    let (ts, _) = emitted(
        "pub fn how_many<I>(walk: I) -> usize where I: Iterator<Item = Token> { walk.count() }",
    );
    assert!(ts.contains("walk.takeRest()"), "`count` consumes the cursor:\n{}", ts);
}

/// And the by-reference/by-value split is a fact about the METHOD, which the
/// `Iterator` table is what holds. The exhausting ones — `rposition`,
/// `try_fold`, `try_for_each` — empty the cursor without consuming it; the
/// short-circuiting ones the cursor answers itself.
#[test]
fn the_iterator_table_says_which_methods_leave_the_cursor_with_its_owner() {
    use crate::native_types::iterator::{cursor_answers_in_place, takes_self_by_reference};
    for (method, arity) in [("rposition", 1), ("try_fold", 2), ("try_for_each", 1)] {
        assert!(takes_self_by_reference(method, arity), "`{}` takes `&mut self`", method);
        assert!(cursor_answers_in_place(method, arity).is_none(), "`{}` walks it all", method);
    }
    for (method, arity, ts) in [
        ("any", 1, "any"),
        ("all", 1, "all"),
        ("find", 1, "find"),
        ("find_map", 1, "findMap"),
        ("position", 1, "position"),
        ("nth", 1, "nth"),
        ("size_hint", 0, "sizeHint"),
    ] {
        assert!(takes_self_by_reference(method, arity), "`{}` takes `&mut self`", method);
        assert_eq!(cursor_answers_in_place(method, arity), Some(ts), "`{}`", method);
    }
    for (method, arity) in [("count", 0), ("collect", 0), ("last", 0), ("fold", 2)] {
        assert!(!takes_self_by_reference(method, arity), "`{}` takes `self`", method);
    }
}

// ── GG1: the ELEMENTS axis — whose values the walk hands out ──

/// GG1: a cursor over `Item = &Token` walks the CALLER's tokens. Built without
/// the mode, dropping it released them and the caller's own `token.drop()` was
/// a double drop.
#[test]
fn a_cursor_over_borrowed_items_is_built_in_the_borrowing_mode() {
    let (ts, _) = emitted(
        "pub fn count_refs<'a, I>(walk: I) -> usize where I: Iterator<Item = &'a Token> \
         { let mut n = 0; for _t in walk { n += 1; } n }\n\
         pub fn caller(tokens: Vec<Token>) -> usize { count_refs(tokens.iter()) }",
    );
    assert!(
        ts.contains("new SeqCursor([...tokens], 'borrow')"),
        "the walk points at the caller's tokens:\n{}",
        ts
    );
}

/// And a cursor over OWNED items keeps the mode it had.
#[test]
fn a_cursor_over_owned_items_is_built_without_a_mode() {
    let (ts, _) = emitted(
        "pub fn first<I>(mut walk: I) -> Option<Token> where I: Iterator<Item = Token> \
         { walk.next() }\n\
         pub fn caller(tokens: Vec<Token>) -> Option<Token> { first(tokens.into_iter()) }",
    );
    assert!(ts.contains("first(new SeqCursor([...tokens]))"), "no mode:\n{}", ts);
    assert!(!ts.contains("'borrow'"), "and nothing says borrowed:\n{}", ts);
}

/// GG4: an EXPLICIT `Iterator` bound wins over an `IntoIterator` beside it. The
/// parameter was rejected as a cursor as soon as any `IntoIterator` bound was
/// present, so `walk.next()` inside the callee named a method an array has not
/// got.
#[test]
fn an_explicit_iterator_bound_wins_over_an_into_iterator_beside_it() {
    for bounds in ["Iterator<Item = Token> + IntoIterator", "IntoIterator + Iterator<Item = Token>"]
    {
        let (ts, _) = emitted(&format!(
            "pub fn first<I>(mut walk: I) -> Option<Token> where I: {} {{ walk.next() }}",
            bounds
        ));
        assert!(
            ts.contains("SeqCursor<Token>"),
            "`{}` is a walk, not a sequence:\n{}",
            bounds,
            ts
        );
    }
}

/// And `IntoIterator` WITHOUT `Iterator` is still the caller's transparent
/// sequence, which the port spreads.
#[test]
fn an_into_iterator_bound_alone_is_still_a_sequence() {
    let (ts, _) = emitted(
        "pub fn how_many<I>(seq: I) -> usize where I: IntoIterator<Item = Token> \
         { seq.into_iter().count() }",
    );
    assert!(
        ts.contains("seq: I") && ts.contains("I extends Iterable<Token>"),
        "the boundary is a sequence the port spreads:\n{}",
        ts
    );
    // It becomes a cursor at the `into_iter()` INSIDE the body, which is where
    // the port's one cursor-building site is.
    assert!(ts.contains("new SeqCursor([...seq])"), "and a walk inside:\n{}", ts);
}

// ── FF4/GG3: the cursor representation through a field and a return ──

/// FF4: a field the declaration walks as a cursor is written as one on the
/// class, and a construction site wraps the value crossing into it. Left as the
/// bare parameter, `Holder { walk: t.into_iter() }` emitted
/// `new Holder([...tokens])` while `pull()` read `this.walk.next()`, a method
/// an array has not got.
#[test]
fn a_field_the_declaration_walks_is_a_cursor_at_the_class_and_at_the_literal() {
    let (ts, _) = emitted(
        "pub struct Holder<I: Iterator<Item = Token>> { pub walk: I }\n\
         impl<I: Iterator<Item = Token>> Holder<I> { \
             pub fn pull(&mut self) -> Option<Token> { self.walk.next() } }\n\
         pub fn stored(tokens: Vec<Token>) -> Option<Token> \
         { let mut h = Holder { walk: tokens.into_iter() }; h.pull() }",
    );
    assert!(ts.contains("readonly walk: SeqCursor<Token>;"), "the property:\n{}", ts);
    assert!(ts.contains("constructor(walk: SeqCursor<Token>)"), "the parameter:\n{}", ts);
    assert!(
        ts.contains("new Holder(new SeqCursor([...tokens]))"),
        "and the value crossing into it:\n{}",
        ts
    );
}

/// A field of the SAME struct that walks nothing keeps its own type.
#[test]
fn a_field_that_walks_nothing_is_not_written_as_a_cursor() {
    let (ts, _) = emitted(
        "pub struct Pair<I: Iterator<Item = Token>> { pub walk: I, pub n: i64 }\n\
         pub fn built(tokens: Vec<Token>, n: i64) -> Pair<std::vec::IntoIter<Token>> \
         { Pair { walk: tokens.into_iter(), n } }",
    );
    assert!(ts.contains("readonly n: bigint;"), "the plain field:\n{}", ts);
    assert!(ts.contains("readonly walk: SeqCursor<Token>;"), "and the walk:\n{}", ts);
}

/// GG3: a function that RETURNS an opaque iterator says `SeqCursor` at the
/// return position. Written as the bare parameter, the caller's local had a
/// type the engine could say nothing about — nothing released the cursor.
#[test]
fn a_returned_cursor_is_spelled_as_one() {
    let (ts, _) = emitted(
        "pub fn identity<I>(walk: I) -> I where I: Iterator<Item = Token> { walk }",
    );
    assert!(
        ts.contains("walk: SeqCursor<Token>): SeqCursor<Token>"),
        "both positions say the same thing:\n{}",
        ts
    );
}

/// GG7: parentheses are punctuation and change no ownership, so a
/// parenthesised `by_ref()` is still a reborrow — and an owning adaptor above
/// one is refused exactly as it is above the unparenthesised spelling.
#[test]
fn a_parenthesised_reborrow_is_still_a_reborrow() {
    for spelling in ["it.by_ref()", "(it.by_ref())"] {
        let (ts, _) = emitted(&format!(
            "pub fn walked(tokens: Vec<Token>) -> usize \
             {{ let mut it = tokens.into_iter(); let n = {}.filter(|t| t.n > 0).count(); \
                it.count() + n }}",
            spelling
        ));
        assert!(
            ts.contains("unsupported("),
            "`{}` above an owning adaptor is refused:\n{}",
            spelling,
            ts
        );
    }
}

/// FF9: ONE spelling for a bound, wherever it is written. A class's generics
/// are merged from the impl blocks written for it, and a bare `Iterator` there
/// is a name TypeScript's own lib declares with two required arguments.
#[test]
fn a_class_generic_spells_its_bound_the_way_a_function_generic_does() {
    let (ts, _) = emitted(
        "pub struct Holder<I: Iterator<Item = Token>> { pub walk: I }\n\
         impl<I: Iterator<Item = Token>> Holder<I> { pub fn n(&self) -> i64 { 1 } }",
    );
    assert!(ts.contains("class Holder<I extends Iterable<Token>>"), "one spelling:\n{}", ts);
    assert!(!ts.contains("& Iterator>"), "and no bare `Iterator`:\n{}", ts);
}

/// And a bound naming a parameter the CLASS does not declare is not carried:
/// `impl<I: Iterator<Item = R>, R> Holder<I>` names the element through the
/// impl's `R`, which the class's generic list has not got.
#[test]
fn a_merged_bound_that_names_an_undeclared_parameter_is_not_carried() {
    let (ts, _) = emitted(
        "pub struct Holder<I> { pub walk: I }\n\
         impl<I: Iterator<Item = R>, R> Holder<I> { pub fn n(&self) -> i64 { 1 } }",
    );
    assert!(!ts.contains("Iterable<R>"), "nothing names the impl's own `R`:\n{}", ts);
}
