//! The catch-all's own tests: which variants a `_` arm is written out for, what
//! each copy declares and releases, and where the expansion is a chain instead.

use crate::testing::Fixture;

fn built(src: &str) -> Fixture {
    Fixture::build(&[("lib.rs", src)])
}

/// `other => other` on a borrowed enum: the arm is written once per variant
/// the source left to it, and each one hands the subject back.
#[test]
fn a_named_catch_all_stands_for_every_variant_left() {
    let mut f = built(
        "pub enum Order { Less, Equal, Greater }\n\
         pub fn pick(o: Order) -> Order {\n\
           match o { Order::Equal => Order::Less, other => other }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "pick");
    assert!(ts.contains("Less: () => {"), "{}", ts);
    assert!(ts.contains("Greater: () => {"), "{}", ts);
    assert_eq!(ts.matches("const other = o;").count(), 2, "{}", ts);
}

/// The `_` arm of a consuming match reads the subject the arms above it did
/// not take. `intoMatch` has moved it, so the arm builds the same value
/// again out of the variant it matched and the payload it was handed.
#[test]
fn a_wildcard_arm_of_a_consuming_match_rebuilds_the_subject() {
    let mut f = built(
        "pub struct Inner;\n\
         pub enum Wrapped { One(Inner), Two }\n\
         pub enum Outer { Held(Inner), Whole(Wrapped) }\n\
         pub fn lift(w: Wrapped) -> Outer {\n\
           match w { Wrapped::One(i) => Outer::Held(i), _ => Outer::Whole(w) }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "lift");
    assert!(ts.contains("w.intoMatch({"), "{}", ts);
    assert!(ts.contains("Two: (v) => {"), "{}", ts);
    assert!(ts.contains("const w = new Wrapped('Two', v);"), "{}", ts);
    assert!(ts.contains("new Outer('Whole', { _0: w })"), "{}", ts);
}

/// A `_` arm over a BORROWED enum whose variants carry nothing needs no
/// payload: the enum stays whole and there is nothing in it to own.
#[test]
fn a_wildcard_arm_over_a_borrowed_payload_free_enum_takes_no_payload() {
    let mut f = built(
        "pub enum Step { A, B, C, D }\n\
         pub fn rank(s: &Step) -> u32 {\n\
           match s { Step::A => 1, Step::B => 2, _ => 0 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "rank");
    assert!(ts.contains("C: () => 0,"), "{}", ts);
    assert!(ts.contains("D: () => 0,"), "{}", ts);
}

/// PREMISE CHANGED 2026-09-04: the test this replaces asserted that a `_`
/// arm which reads nothing takes no payload, full stop. Under `intoMatch`
/// that is a leak — the payload is handed over and nobody receives it — so
/// the rule is now that a CONSUMING arm always takes the payload and owns
/// all of it, and only a borrowing arm over a payload-free enum can decline
/// it.
#[test]
fn a_consuming_wildcard_arm_releases_the_payload_it_reads_nothing_of() {
    let mut f = built(
        "pub struct Inner;\n\
         pub enum Step { Taken(Inner), Rest(Inner) }\n\
         pub fn rank(s: Step) -> u32 {\n\
           match s { Step::Taken(i) => 1, _ => 0 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "rank");
    assert!(ts.contains("s.intoMatch({"), "{}", ts);
    assert!(ts.contains("Rest: (v) => {"), "{}", ts);
    assert!(ts.contains("dropUnbound(v, []);"), "{}", ts);
}

/// A named arm that ignores its payload owns it just the same: `intoMatch`
/// hands the whole thing over and releases nothing of its own.
#[test]
fn a_named_arm_that_ignores_its_payload_releases_it() {
    let mut f = built(
        "pub struct Inner;\n\
         pub enum Step { Taken(Inner), Rest(Inner) }\n\
         pub fn rank(s: Step) -> u32 {\n\
           match s { Step::Taken(_) => 1, Step::Rest(i) => 2 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "rank");
    assert!(ts.contains("Taken: (v) => {"), "{}", ts);
    assert!(ts.contains("dropUnbound(v, []);"), "{}", ts);
}

/// A catch-all that binds the scrutinee's own name binds the value once.
/// `match e { E::Taken(t) => .., e => e }` wrote `const e` twice, which no
/// JavaScript engine will load.
#[test]
fn a_catch_all_that_shadows_the_subject_declares_it_once() {
    let mut f = built(
        "pub struct Inner;\n\
         pub enum E { Taken(Inner), Rest(Inner) }\n\
         pub fn keep(e: E) -> E {\n\
           match e { E::Taken(t) => E::Rest(t), e => e }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "keep");
    assert_eq!(ts.matches("const e = new E('Rest', v);").count(), 1, "{}", ts);
}

/// A borrowing catch-all that binds the scrutinee's own name declares
/// nothing: `const e = e;` reads the name it is declaring.
#[test]
fn a_borrowing_catch_all_that_shadows_the_subject_declares_nothing() {
    let mut f = built(
        "pub enum E { A, B, C }\n\
         pub fn count(e: &E) -> u32 {\n\
           match e { E::A => 1, e => 0 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "count");
    assert!(!ts.contains("const e = e;"), "{}", ts);
}

/// A catch-all that binds a BORROWED subject does not release it: the
/// caller still owns it, and the arm dropping it was a double drop.
#[test]
fn a_catch_all_binding_a_borrowed_subject_releases_nothing() {
    let mut f = built(
        "pub struct Inner;\n\
         pub enum E { A(Inner), B(Inner), C(Inner) }\n\
         pub struct Holder { pub choice: E }\n\
         pub fn pick(h: &Holder) -> u32 {\n\
           match &h.choice { E::A(_) => 1, other => 0 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "pick");
    assert!(!ts.contains("other.drop()"), "{}", ts);
}

/// An arm written after the catch-all can never run, and says so.
#[test]
fn an_arm_after_the_catch_all_is_reported() {
    let mut f = built(
        "pub enum Step { A, B }\n\
         pub fn rank(s: &Step) -> u32 {\n\
           match s { Step::A => 1, _ => 0, Step::B => 2 }\n\
         }",
    );
    let _ = f.translated_method("lib.rs", "rank");
    assert!(
        f.messages().iter().any(|m| m.contains("never run")),
        "{:?}",
        f.messages()
    );
}

/// A subject the engine cannot type has no variant list to write the arm
/// against, and that is said rather than passed over.
#[test]
fn a_catch_all_over_an_untyped_subject_is_reported() {
    let mut f = built(
        "pub fn rank<T>(s: &T) -> u32 {\n\
           match s { Step::A => 1, _ => 0 }\n\
         }",
    );
    let _ = f.translated_method("lib.rs", "rank");
    assert!(
        f.messages().iter().any(|m| m.contains("names no variant")),
        "{:?}",
        f.messages()
    );
}

/// A match whose arms all name a variant is still the runtime's own match.
#[test]
fn a_match_with_no_catch_all_is_left_alone() {
    let mut f = built(
        "pub enum Step { A, B }\n\
         pub fn rank(s: &Step) -> u32 {\n\
           match s { Step::A => 1, Step::B => 2 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "rank");
    assert!(ts.starts_with("return s.match({"), "{}", ts);
}

/// PREMISE CHANGED 2026-09-04: "every variant is already named" used to be
/// read off the variant NAMES alone, so an arm that named a variant and
/// tested inside it counted as covering the whole of it and the catch-all
/// was deleted — leaving the values that arm does not match with no arm at
/// all. An arm covers its variant only when it matches every value of it,
/// which is the premise the test below now states, and the refutable case
/// is the test after it.
///
/// A catch-all that stands for nothing — every variant already covered — is
/// left out, because Rust cannot reach it either.
#[test]
fn a_catch_all_with_nothing_left_to_cover_is_dropped() {
    let mut f = built(
        "pub enum Step { A, B }\n\
         pub fn rank(s: &Step) -> u32 {\n\
           match s { Step::A => 1, Step::B => 2, _ => 0 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "rank");
    assert!(!ts.contains("return 0;"), "{}", ts);
}

/// An arm that tests INSIDE its variant does not cover the variant, so the
/// catch-all still stands for the values it does not match. The runtime's
/// match cannot express "test the payload, and fall through if it fails",
/// so a borrowing match goes to the if-chain, which can.
#[test]
fn an_arm_that_tests_inside_its_variant_does_not_delete_the_catch_all() {
    let mut f = built(
        "pub enum Lit { S, I }\n\
         pub enum Ex { Literal(Lit), Path }\n\
         pub fn rank(e: &Ex) -> u32 {\n\
           match e { Ex::Literal(Lit::I) => 7, Ex::Path => 1, _ => 99 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "rank");
    assert!(!ts.contains(".match({"), "{}", ts);
    assert!(ts.contains("99"), "{}", ts);
    assert!(ts.contains("is('Literal')"), "{}", ts);
    assert!(ts.contains("is('I')"), "{}", ts);
}

/// PREMISE CHANGED 2026-09-05 (fixpass4 item 1): the test this replaces
/// asserted that a CONSUMING match with an arm testing inside its variant
/// was reported and the arm then ran for every value of that variant — the
/// if-chain the borrowing form is rewritten to reads the payload without
/// marking the enum moved, so it was not available here. The arm chain is:
/// the key keeps `intoMatch`'s payload and the branches inside it make the
/// test the key cannot, with the catch-all's body as the last `else`.
///
/// PREMISE CHANGED AGAIN 2026-09-06 (slice 4 item 4, K4): the arm binds
/// `i: Inner` OUT of the `Lit` the `Literal` variant holds and nothing
/// releases the `Lit`, which the `Result` side has always refused — so the
/// arm's BODY is a hole. The chain around it is what this still asserts.
#[test]
fn a_consuming_match_that_tests_inside_a_variant_is_a_chain() {
    let mut f = built(
        "pub struct Inner;\n\
         pub enum Lit { S(Inner), I(Inner) }\n\
         pub enum Ex { Literal(Lit), Path(Inner) }\n\
         pub fn rank(e: Ex) -> u32 {\n\
           match e { Ex::Literal(Lit::I(i)) => 7, _ => 99 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "rank");
    assert!(ts.contains("e.intoMatch({"), "{}", ts);
    // The test the key used to be unable to make.
    assert!(ts.contains("Literal: (v) => {"), "{}", ts);
    assert!(ts.contains("if (v._0.is('I')) {"), "{}", ts);
    // K4: the arm takes `i` out of the `Lit` and leaves the rest.
    let release = ts.find("dropUnbound(v, []);").expect(&ts);
    let throw = ts.find("unsupported(").expect(&ts);
    assert!(release < throw, "the refusal releases what it holds first:\n{}", ts);
    // and the catch-all's body where the test fails.
    assert!(ts.contains("} else {"), "{}", ts);
    assert!(ts.contains("return 99;"), "{}", ts);
    assert!(
        !f.messages().iter().any(|m| m.contains("nowhere to fall through to")),
        "the gap that report named is closed: {:?}",
        f.messages()
    );
}
