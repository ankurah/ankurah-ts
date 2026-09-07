//! An `if let` whose pattern takes the payload OUT of the value it tests.
//!
//! For: such a pattern CONSUMES the enum — Rust takes the binding out, drops
//! the fields the pattern did not name, and the value the `if let` read is
//! gone. The port has exactly one construct that does that, `intoMatch`, and it
//! belongs to the match writer; an `if` could only read the payload out and
//! leave the enum standing, so the block that owned it released it a second
//! time with the taken binding still inside.
//!
//! Rust's own desugaring of `if let PAT = e { A } else { B }` is
//! `match e { PAT => A, _ => B }`, so that is what the port writes.

use crate::testing::Fixture;

/// ankql's `if let Predicate::Comparison { right: val, .. } = *inner_left`, in
/// one crate: a struct variant, a `..` leaving two fields unnamed, and a `Box`
/// the source moves out of.
fn taken_out_of_a_box() -> String {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Op { pub n: u32 }\n\
         pub struct Leaf { pub n: u32 }\n\
         pub enum Node {\n\
           Pair { left: Box<Node>, op: Op, right: Box<Node> },\n\
           End(Leaf),\n\
         }\n\
         pub fn look(l: &Leaf) -> u32 { l.n }\n\
         pub fn probe(node: Box<Node>) -> u32 {\n\
           if let Node::Pair { right: val, .. } = *node {\n\
             return 1;\n\
           }\n\
           0\n\
         }",
    )]);
    f.translated_method("lib.rs", "probe")
}

#[test]
fn a_consuming_if_let_is_written_as_a_match() {
    let ts = taken_out_of_a_box();
    assert!(
        ts.contains(".intoMatch"),
        "the construct that consumes an enum is `intoMatch`, and an `if` is not one:\n{}",
        ts
    );
    assert!(
        !ts.contains("unsupported("),
        "and nothing here is refused:\n{}",
        ts
    );
}

#[test]
fn the_fields_the_pattern_did_not_take_are_released() {
    let ts = taken_out_of_a_box();
    assert!(
        ts.contains("dropUnbound("),
        "`left` and `op` are dropped where the pattern moves `right` out, which is what \
         Rust does:\n{}",
        ts
    );
}

#[test]
fn the_box_the_pattern_moved_out_of_is_not_released_again() {
    let ts = taken_out_of_a_box();
    assert!(
        !ts.contains("dropOwned(node)") && !ts.contains("node.drop()"),
        "`*node` is Rust's deref-move: the box goes with the value taken out of it, and \
         `intoMatch` has already marked it moved:\n{}",
        ts
    );
}

/// The other side of the same question: `*guard` on a `MutexGuard` is the
/// `Deref` trait, which BORROWS. Counting it as a move left the guard released
/// by nobody.
#[test]
fn a_deref_through_a_guard_moves_nothing() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Mutex;\n\
         pub struct Held { pub cell: Mutex<u32> }\n\
         impl Held {\n\
           pub fn read(&self) -> u32 {\n\
             let g = self.cell.lock().unwrap();\n\
             *g\n\
           }\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "read");
    assert!(
        ts.contains("g.drop();"),
        "the guard is still the block's, so the block still releases it:\n{}",
        ts
    );
}

/// A field moved out of a value the PORT holds in a temporary is a partial
/// move too: Rust's temporary knows the field is gone, and the port's `_tN`
/// cascade does not.
#[test]
fn a_field_taken_out_of_a_held_temporary_comes_out_of_it() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "#[derive(Clone)]\n\
         pub struct Inner { pub n: u32 }\n\
         #[derive(Clone)]\n\
         pub struct Holder { pub inner: Inner, pub tag: u32 }\n\
         pub fn eat(i: Inner) -> u32 { i.n }\n\
         pub fn probe(h: Holder) -> u32 {\n\
           let taken = h.clone().inner;\n\
           eat(taken)\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "probe");
    assert!(
        ts.contains(".takeField('inner')"),
        "the clone's own release would otherwise cascade into the field `eat` took:\n{}",
        ts
    );
}

/// And a field read that is not a move keeps its plain property read: reading
/// a number out of a temporary takes nothing from anybody.
#[test]
fn a_copy_field_of_a_temporary_is_still_read_in_place() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "#[derive(Clone)]\n\
         pub struct Inner { pub n: u32 }\n\
         #[derive(Clone)]\n\
         pub struct Holder { pub inner: Inner, pub tag: u32 }\n\
         pub fn probe(h: Holder) -> u32 { h.clone().tag }",
    )]);
    let ts = f.translated_method("lib.rs", "probe");
    assert!(
        !ts.contains("takeField"),
        "a `u32` has no drop glue for a cascade to reach twice:\n{}",
        ts
    );
}

/// An OR-pattern over one of the port's enums is one ARM per alternative.
///
/// Routed by the leaf of the whole pattern, `Shape::One(t) | Shape::Two(t, _)`
/// was neither a tuple struct nor a path, so the match went to the if-chain:
/// `s.value` read in place, nothing marked moved, `s` released by nobody and
/// the `Two` payload the pattern did not name released by nobody either. The
/// budget FELL while the code got worse, because the report that named the gap
/// went with it.
#[test]
fn an_or_pattern_consuming_if_let_is_a_match_arm_per_alternative() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub enum Shape { One(Token), Two(Token, u32), Nothing }\n\
         pub fn take(s: Shape) -> u32 { \
         if let Shape::One(t) | Shape::Two(t, _) = s { t.n } else { 0 } }",
    )]);
    let ts = f.translated_method("lib.rs", "take");
    assert!(ts.contains("s.intoMatch({"), "the enum is consumed:\n{}", ts);
    assert!(ts.contains("One: (v) =>"), "one arm per alternative:\n{}", ts);
    assert!(ts.contains("Two: (v) =>"), "one arm per alternative:\n{}", ts);
    assert!(!ts.contains("s.is('One')"), "and no in-place test is left:\n{}", ts);
}

/// A NESTED ordinary struct is destructured, not asked which variant it is.
///
/// `Duo::A(Token { n })` wrote `v._0.is('Token')` — a method no struct has —
/// so the branch threw and the value it had opened stayed live.
#[test]
fn a_nested_struct_pattern_is_destructured_directly() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub enum Duo { A(Token), B }\n\
         pub fn take(d: Duo) -> u32 { if let Duo::A(Token { n }) = d { n } else { 0 } }",
    )]);
    let ts = f.translated_method("lib.rs", "take");
    assert!(ts.contains("const { n } = v._0;"), "the fields stand on the object:\n{}", ts);
    assert!(!ts.contains(".is('Token')"), "a struct has no variant test:\n{}", ts);
}

/// A crate's own `Ok` variant is its own, and is opened as one.
///
/// Recognised by the leaf `Ok`, `Outcome::Ok(token)` was written as
/// `o.isOk()`/`o.unwrap()` on a class carrying `is`, `value` and `intoMatch`
/// and neither of those: the call threw and the enum stayed live.
#[test]
fn a_user_enum_spelling_ok_is_not_the_runtime_result() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub enum Outcome { Ok(Token), Other }\n\
         pub fn take(o: Outcome) -> u32 { if let Outcome::Ok(t) = o { t.n } else { 0 } }",
    )]);
    let ts = f.translated_method("lib.rs", "take");
    assert!(ts.contains("o.intoMatch({"), "its own enum, consumed as one:\n{}", ts);
    assert!(ts.contains("Ok: (v) =>"), "its own variant:\n{}", ts);
    assert!(!ts.contains("isOk()"), "and not the wrapper's:\n{}", ts);
    assert!(!ts.contains("unwrap()"), "and not the wrapper's:\n{}", ts);
}

/// A GENUINE `Result` the engine reaches through a projection is still the
/// wrapper.
///
/// The narrow question is "did the corpus declare this enum", not "did the
/// engine resolve this to `std::result::Result`": asked the second way,
/// `storage-indexeddb/scanner.rs`'s `stream.next().await?` came out
/// `result.is('Ok')`.
#[test]
fn a_result_the_engine_reads_through_a_bound_is_still_the_wrapper() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn pick<I: Iterator<Item = Result<Token, Token>>>(mut i: I) -> u32 { \
         if let Some(r) = i.next() { if let Ok(t) = r { t.n } else { 0 } } else { 0 } }",
    )]);
    let ts = f.translated_method("lib.rs", "pick");
    assert!(ts.contains("isOk()"), "the wrapper is opened as one:\n{}", ts);
    assert!(!ts.contains(".is('Ok')"), "and not as a variant:\n{}", ts);
}

/// A pattern that reaches INSIDE a tuple element to take a payload says so.
///
/// `if let (Duo::A(token), _) = pair` takes `token` out of `pair[0]` by reading
/// `.value`, which marks nothing. The tuple's own release used to run
/// underneath and drop the payload a second time; now the match consumes and
/// the `Duo` wrapper is released by nobody. The port has no lowering that
/// reaches one level down, so this is a report rather than either wrong answer.
#[test]
fn a_payload_taken_from_a_tuple_element_is_reported() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub enum Duo { A(Token), B }\n\
         pub fn take(pair: (Duo, u32)) -> u32 { \
         if let (Duo::A(t), _) = pair { t.n } else { 0 } }",
    )]);
    let ts = f.translated_method("lib.rs", "take");
    assert!(!ts.contains("dropOwned(pair)"), "the payload is not dropped twice:\n{}", ts);
    assert!(
        f.messages().iter().any(|m| m.contains("reaches inside a tuple element")),
        "and the gap is named: {:?}",
        f.messages()
    );
}

/// A let-chain GUARD that fails runs the source's `else` and releases what the
/// pattern took.
///
/// Written as a nested `if` with no `else` of its own, the guard-false path
/// fell out of BOTH branches: the function returned `undefined` where the
/// source returns the `else`'s value, and the payload `unwrap()` had already
/// taken was owned by nobody.
#[test]
fn a_false_let_chain_guard_runs_the_else_and_releases_the_payload() {
    for subject in ["Result<Token, u32>", "Option<Token>"] {
        let pattern = match subject.starts_with("Result") {
            true => "Ok(token)",
            false => "Some(token)",
        };
        let mut f = Fixture::build(&[(
            "lib.rs",
            &format!(
                "pub struct Token {{ pub n: u32 }}\n\
                 impl Drop for Token {{ fn drop(&mut self) {{}} }}\n\
                 pub fn take(r: {}, allow: bool) -> u32 {{ \
                 if let {} = r && allow {{ token.n }} else {{ 7 }} }}",
                subject, pattern
            ),
        )]);
        let ts = f.translated_method("lib.rs", "take");
        let guard = ts.split("if (allow) {").nth(1).unwrap_or_default();
        assert!(guard.contains("} else {"), "the guard has an else:\n{}", ts);
        assert!(guard.contains("token.drop();"), "which releases the payload:\n{}", ts);
        assert_eq!(ts.matches("return 7;").count(), 2, "and runs the else body:\n{}", ts);
    }
}

/// And a guard whose failure has nothing to release and no `else` to run
/// writes no `else` at all.
#[test]
fn a_guard_over_borrowed_bindings_writes_no_empty_else() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn take(r: &Option<Token>, allow: bool) -> u32 { \
         let mut total = 0; if let Some(t) = r && allow { total = t.n; } total }",
    )]);
    let ts = f.translated_method("lib.rs", "take");
    assert!(!ts.contains("} else {\n      }"), "no empty else:\n{}", ts);
}
