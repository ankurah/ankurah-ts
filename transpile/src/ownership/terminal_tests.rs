//! Who owns the elements an iteration walked. F1, E11, F4/E12, G1.
//!
//! For: Rust's `into_iter().find(p)` OWNS what it walks — it hands back the
//! element it selected and drops every other one — and the port wrote it as a
//! reading helper over an array somebody else releases. Two wrong answers came
//! out of that, and which one depended on whether Rust's signature happens to
//! say `self` or `&mut self` about the ITERATOR, a distinction that says
//! nothing about the items.

use crate::testing::Fixture;

const TOKEN: &str = "pub struct Token(pub u32);\n\
                     impl Drop for Token { fn drop(&mut self) { } }\n";

fn body(rust: &str, method: &str) -> String {
    let mut f = Fixture::build(&[("lib.rs", &format!("{}{}", TOKEN, rust))]);
    f.translated_method("lib.rs", method)
}

/// A terminal whose Rust receiver is `&mut self` was hoisted and released, so
/// the sequence was walked by a reader and then released WHOLE.
///
/// The reviewer's probe: the closure drops the element Rust gave it, and the
/// `finally` drops it again — `OwnershipFatal`. And `find` handed back an
/// element of a sequence the same `finally` then released, so the caller was
/// given a dropped value.
#[test]
fn a_consuming_terminal_releases_what_it_does_not_hand_back() {
    let ts = body(
        "pub fn position_of(tokens: Vec<Token>) -> Option<usize> {\n\
           tokens.into_iter().position(|token| { drop(token); true })\n\
         }",
        "position_of",
    );
    assert!(ts.contains("iterPositionOwned("), "the owned terminal is what is written:\n{}", ts);
    assert!(!ts.contains("dropOwned("), "the sequence is the terminal's now:\n{}", ts);

    let found = body(
        "pub fn find_one(tokens: Vec<Token>) -> Option<Token> {\n\
           tokens.into_iter().find(|t| t.0 == 1)\n\
         }",
        "find_one",
    );
    assert!(found.contains("iterFindOwned("), "{}", found);
    assert!(!found.contains("dropOwned("), "{}", found);
}

/// A terminal whose Rust receiver is `self` was not hoisted at all, so every
/// element it did not hand back simply leaked.
#[test]
fn a_terminal_that_took_the_iterator_still_owes_the_losers() {
    for (rust, method, helper) in [
        (
            "pub fn biggest(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().max_by_key(|t| t.0) }",
            "biggest",
            "iterMaxByKeyOwned(",
        ),
        (
            "pub fn smallest(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().min_by_key(|t| t.0) }",
            "smallest",
            "iterMinByKeyOwned(",
        ),
        (
            "pub fn folded(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().reduce(|a, _b| a) }",
            "folded",
            "iterReduceOwned(",
        ),
        (
            "pub fn last_of(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().last() }",
            "last_of",
            "iterLastOwned(",
        ),
    ] {
        let ts = body(rust, method);
        assert!(ts.contains(helper), "`{}` was expected to write `{}`:\n{}", method, helper, ts);
    }
}

/// A BORROWED chain is unchanged: `iter()` hands out `&T`, the elements are
/// somebody else's, and the reading helper is what the sequence's owner needs.
#[test]
fn a_borrowed_chain_keeps_the_reading_helper() {
    let ts = body(
        "pub fn borrowed_find(tokens: &Vec<Token>) -> Option<&Token> {\n\
           tokens.iter().find(|t| t.0 == 1)\n\
         }",
        "borrowed_find",
    );
    assert!(ts.contains("iterFind("), "{}", ts);
    assert!(!ts.contains("iterFindOwned("), "a borrowed chain owns nothing:\n{}", ts);
}

/// A sequence of numbers has nothing to release, so the ownership mode changes
/// nothing about what is written.
#[test]
fn elements_with_no_drop_glue_keep_the_reading_helper() {
    let ts = body(
        "pub fn first_even(ns: Vec<u32>) -> Option<u32> { ns.into_iter().find(|n| *n % 2 == 0) }",
        "first_even",
    );
    assert!(ts.contains("iterFind("), "{}", ts);
    assert!(!ts.contains("iterFindOwned("), "{}", ts);
}

/// `slice::last(&self)` and `Iterator::last(self)` are two methods of one name.
/// The resolution says which was found; the name says nothing.
#[test]
fn a_slice_reader_of_the_same_name_is_not_the_consuming_terminal() {
    let ts = body(
        "pub fn last_of(tokens: &Vec<Token>) -> Option<&Token> { tokens.last() }",
        "last_of",
    );
    assert!(ts.contains("iterLast("), "{}", ts);
    assert!(!ts.contains("iterLastOwned("), "a borrow of somebody's slice releases nothing:\n{}", ts);
}

/// A consuming terminal on a NAMED iterator leaves part of the sequence behind,
/// and the port's array holds both halves afterwards. Refused (R12) rather than
/// released in whichever direction happens to be wrong.
#[test]
fn a_consuming_terminal_on_a_named_iterator_is_refused() {
    let ts = body(
        "pub fn named(tokens: Vec<Token>) -> Option<Token> {\n\
           let mut it = tokens.into_iter();\n\
           it.find(|t| t.0 == 1)\n\
         }",
        "named",
    );
    assert!(ts.contains("unsupported("), "{}", ts);
    assert!(ts.contains("cannot say which of its elements"), "{}", ts);
    // J4: the hole throws before anything is handed away, so the block still
    // holds the sequence and still releases it.
    assert!(ts.contains("dropOwned(it)"), "the block keeps what the hole did not take:\n{}", ts);
}

/// E11: `(&v).into_iter()` is `IntoIterator for &Vec<T>`, whose `Item` is a
/// BORROW.
///
/// `&x` as an expression types as `x` — emission erases borrows — and the same
/// erasure ran before the method probe, so the receiver of `(&v).into_iter()`
/// was `Vec<T>` and the by-value impl answered. The loop then released every
/// element the caller still owned: a double drop where the block released them
/// too, and a release of somebody else's elements where it did not.
#[test]
fn a_borrow_written_as_a_call_owns_nothing() {
    let ts = body(
        "pub fn widths(tokens: Vec<Token>) -> u32 {\n\
           let mut n = 0;\n\
           for t in (&tokens).into_iter() { n += t.0; }\n\
           n\n\
         }",
        "widths",
    );
    assert!(ts.contains("dropOwned(tokens)"), "the block still releases the vector:\n{}", ts);
    assert!(!ts.contains("t.drop()"), "the loop released the caller's elements:\n{}", ts);

    // The `.iter()` spelling of the same thing, which was already right.
    let iter = body(
        "pub fn widths(tokens: Vec<Token>) -> u32 {\n\
           let mut n = 0;\n\
           for t in tokens.iter() { n += t.0; }\n\
           n\n\
         }",
        "widths",
    );
    assert!(!iter.contains("t.drop()"), "{}", iter);
}

/// F4/E12: `iter_mut` had no lowering at all and came out as `xs.iterMut()`,
/// a method no array and no map declares — live at `core/node.ts` and
/// `core/property/backend/lww.ts`. It hands out `&mut T`, which the port writes
/// as the element object itself; the elements stay the caller's.
#[test]
fn iter_mut_is_the_sequence_where_the_element_is_an_object() {
    let ts = body(
        "pub fn bump(tokens: &mut Vec<Token>) { for t in tokens.iter_mut() { t.0 += 1; } }",
        "bump",
    );
    assert!(ts.contains("[...tokens]"), "{}", ts);
    assert!(!ts.contains("iterMut"), "a method no array declares:\n{}", ts);
    assert!(!ts.contains(".drop()"), "a borrowed iteration releases nothing:\n{}", ts);

    let map = body(
        "pub fn bump(m: &mut std::collections::HashMap<String, Token>) {\n\
           for (_k, v) in m.iter_mut() { v.0 += 1; }\n\
         }",
        "bump",
    );
    assert!(map.contains("[...m]"), "{}", map);
    assert!(!map.contains("iterMut"), "{}", map);
}

/// The port has no `&mut`, so a loop writes THROUGH only because the variable
/// and the slot are the same object. Over a number, a string or a `bigint` the
/// variable is a copy and the write is lost, which is refused rather than
/// written silently.
#[test]
fn iter_mut_over_a_value_the_port_copies_is_refused() {
    for (rust, method) in [
        ("pub fn bump(ns: &mut Vec<u32>) { for n in ns.iter_mut() { *n += 1; } }", "bump"),
        ("pub fn clear(bs: &mut Vec<u8>) { for b in bs.iter_mut() { *b = 0; } }", "clear"),
        (
            "pub fn bump(m: &mut std::collections::HashMap<String, u32>) {\n\
               for (_k, v) in m.iter_mut() { *v += 1; }\n\
             }",
            "bump",
        ),
    ] {
        let ts = body(rust, method);
        assert!(ts.contains("unsupported("), "`{}` was written silently:\n{}", method, ts);
        assert!(ts.contains("the loop would bind a COPY"), "{}", ts);
    }
}

/// G1: `IntoIterator::into_iter` on a type PARAMETER carrying the bound is the
/// same spread it is on a `Vec`.
///
/// The port materialises an iterator as a JavaScript array, so the spread is
/// what `into_iter` is on every receiver. A bare type parameter fell between
/// the shape table's arms and came out as `values.intoIter()` — a method
/// nothing declares, and the cause of all seven failures in ankql's
/// `ast.test.ts`.
#[test]
fn into_iter_on_a_bounded_parameter_is_the_spread() {
    let ts = body(
        "pub fn total<I: IntoIterator<Item = u32>>(values: I) -> u32 {\n\
           let mut n = 0;\n\
           for v in values.into_iter() { n += v; }\n\
           n\n\
         }",
        "total",
    );
    assert!(ts.contains("[...values]"), "{}", ts);
    assert!(!ts.contains("intoIter"), "a method nothing declares:\n{}", ts);
}

/// O2: what the terminal RELEASES is its callback, and only where the source
/// handed it over. `find(&mut p)` type-checks through `impl FnMut for &mut F`,
/// so what the terminal takes by value is the reference: dropping it does
/// nothing and `p` is still the caller's to call again. Released regardless,
/// the next call read captures that were gone and the caller's own release
/// dropped the closure a second time.
#[test]
fn a_terminal_releases_its_callback_only_where_the_source_handed_it_over() {
    let owned = body(
        "pub fn find_one(tokens: Vec<Token>, want: u32) -> Option<Token> {\n\
           tokens.into_iter().find(move |t| t.0 == want)\n\
         }",
        "find_one",
    );
    assert!(owned.contains("iterFindOwned("), "{}", owned);
    assert!(!owned.contains("'borrow'"), "handed over, so the terminal owes it:\n{}", owned);

    let borrowed = body(
        "pub fn find_one(tokens: Vec<Token>, want: u32) -> Option<Token> {\n\
           let mut p = move |t: &Token| t.0 == want;\n\
           tokens.into_iter().find(&mut p)\n\
         }",
        "find_one",
    );
    assert!(
        borrowed.contains("iterFindOwned([...tokens], p, 'borrow')"),
        "borrowed, so the terminal leaves it alone:\n{}",
        borrowed
    );
}

/// The reading family takes its callback the same two ways.
#[test]
fn the_reading_family_carries_the_callback_mode_too() {
    let ts = body(
        "pub fn find_one(tokens: &Vec<Token>, want: u32) -> bool {\n\
           let p = move |t: &&Token| t.0 == want;\n\
           tokens.iter().find(&p).is_some()\n\
         }",
        "find_one",
    );
    assert!(ts.contains("iterFind([...tokens], p, 'borrow')"), "{}", ts);
}

/// O5: `Iterator::by_ref(&mut self) -> &mut Self` is a borrowed VIEW of the
/// iterator it was called on, so a consuming terminal reached through it names
/// that iterator and is refused exactly as `(&mut it).find(..)` is. The
/// emitted `it.byRef()` was a method no array declares, and the refusal never
/// saw the name.
#[test]
fn a_consuming_terminal_through_by_ref_names_the_iterator_and_is_refused() {
    let ts = body(
        "pub fn first_of(tokens: Vec<Token>) -> Option<Token> {\n\
           let mut it = tokens.into_iter();\n\
           it.by_ref().find(|t| t.0 > 0)\n\
         }",
        "first_of",
    );
    assert!(!ts.contains("byRef"), "no method of that name is written:\n{}", ts);
    assert!(
        ts.contains("consumes the elements it walks and leaves the rest in the iterator"),
        "the same refusal `(&mut it).find(..)` gets:\n{}",
        ts
    );
    assert!(ts.contains("dropOwned(it)"), "and the block keeps the receiver:\n{}", ts);
}

/// On a BORROWED chain nothing is consumed, so `by_ref` is the identity.
#[test]
fn by_ref_on_a_borrowed_chain_is_the_identity() {
    let ts = body(
        "pub fn first_of(tokens: &Vec<Token>) -> Option<&Token> {\n\
           let mut it = tokens.iter();\n\
           it.by_ref().find(|t| t.0 > 0)\n\
         }",
        "first_of",
    );
    assert!(ts.contains("iterFind(it, "), "the view is the receiver itself:\n{}", ts);
    assert!(!ts.contains("byRef"), "{}", ts);
}

/// O3/O4: an eager adaptor that DISCARDS elements owns what it discards, and
/// Rust drops it — `Filter` drops what its predicate rejected, `Skip` the
/// prefix, `Take` the tail, `StepBy` what it stepped over. Written as array
/// operations they lost them, and the consuming terminal below could not
/// release what the adaptor had already erased.
#[test]
fn an_eager_adaptor_over_owned_elements_releases_what_it_discards() {
    for (rust, method, helper) in [
        (
            "pub fn f(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().filter(|t| t.0 > 0).last() }",
            "f",
            "filterOwned(",
        ),
        (
            "pub fn f(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().skip(1).last() }",
            "f",
            "skipOwned(",
        ),
        (
            "pub fn f(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().take(1).last() }",
            "f",
            "takeOwned(",
        ),
        (
            "pub fn f(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().step_by(2).last() }",
            "f",
            "stepByOwned(",
        ),
    ] {
        let ts = body(rust, method);
        assert!(ts.contains(helper), "{} is what {} writes:\n{}", helper, rust, ts);
    }
}

/// A borrowed chain discards nothing of its own, so the array operations stand.
#[test]
fn a_borrowed_chain_keeps_the_plain_adaptors() {
    let ts = body(
        "pub fn f(tokens: &Vec<Token>) -> Option<&Token> {\n\
           tokens.iter().filter(|t| t.0 > 0).skip(1).last()\n\
         }",
        "f",
    );
    assert!(!ts.contains("Owned("), "nothing here owns anything:\n{}", ts);
    assert!(ts.contains(".filter("), "{}", ts);
    assert!(ts.contains(".slice(1)"), "{}", ts);
}

/// Q1: `next` on a receiver the expression just BUILT has no cursor to be wrong
/// about — the call answers the head, and the iterator, dropped at the end of
/// the statement, drops the rest. On a NAMED iterator it stays refused, because
/// after such a call the port cannot say which elements are still the caller's.
#[test]
fn next_on_a_fresh_receiver_is_the_head_and_on_a_named_one_is_refused() {
    let owned = body(
        "pub fn f(tokens: Vec<Token>) -> Option<Token> { tokens.into_iter().next() }",
        "f",
    );
    assert!(owned.contains("iterFirstOwned("), "the tail goes with the iterator:\n{}", owned);

    let borrowed = body(
        "pub fn f(tokens: &Vec<Token>) -> Option<&Token> { tokens.iter().next() }",
        "f",
    );
    assert!(borrowed.contains("iterFirst("), "a borrowed chain reads through:\n{}", borrowed);

    let named = body(
        "pub fn f(tokens: Vec<Token>) -> Option<Token> {\n\
           let mut it = tokens.into_iter();\n\
           it.next()\n\
         }",
        "f",
    );
    assert!(named.contains("cursor to advance"), "a named iterator is refused:\n{}", named);
}

/// N5: `Option<Option<T>>` has one `null` for two answers, and the refusal
/// missed every reader reached through a BORROWED chain. `iter()` hands out
/// `&T`, so over a `&Vec<Option<u32>>` the element comes back as
/// `&Option<u32>`; asked about the reference rather than about what it points
/// at, the port read it as "not a nullable" and flattened the two `null`s with
/// no diagnostic — while the OWNED spelling of the same reader refused. The
/// test goes through the iterator path, not through `array::translate`.
#[test]
fn a_reader_over_borrowed_options_is_refused_as_the_owned_one_is() {
    let refusal = "is itself an `Option`";
    for (rust, method) in [
        (
            "pub fn f(slots: &Vec<Option<u32>>) -> Option<&Option<u32>> { slots.iter().find(|s| s.is_some()) }",
            "f",
        ),
        (
            "pub fn f(slots: &Vec<Option<u32>>) -> Option<&Option<u32>> { slots.iter().reduce(|a, _b| a) }",
            "f",
        ),
        (
            "pub fn f(slots: &Vec<Option<u32>>) -> Option<&Option<u32>> { slots.iter().min_by_key(|s| s.unwrap_or(0)) }",
            "f",
        ),
        (
            "pub fn f(slots: Vec<Option<u32>>) -> Option<Option<u32>> { slots.into_iter().find(|s| s.is_some()) }",
            "f",
        ),
    ] {
        let mut f = Fixture::build(&[("lib.rs", rust)]);
        let ts = f.translated_method("lib.rs", method);
        assert!(ts.contains(refusal), "refused:\n{}\nfor:\n{}", ts, rust);
    }
    // A plain element still answers.
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub fn f(ns: &Vec<u32>) -> Option<&u32> { ns.iter().find(|n| **n > 7) }",
    )]);
    let plain = f.translated_method("lib.rs", "f");
    assert!(plain.contains("iterFind("), "{}", plain);
    assert!(!plain.contains(refusal), "{}", plain);
}
