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
