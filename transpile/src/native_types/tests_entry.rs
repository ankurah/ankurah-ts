//! `map.entry(k)` and the three ways Rust finishes one.
//!
//! A finisher answers `&'a mut V` in Rust and a write-through `Slot` here. A
//! `*` stores into the map through it; every other position reads the value it
//! points at, and a `Slot` has none of the value's own methods.

use crate::testing::Fixture;

fn body(src: &str, method: &str) -> String {
    let mut f = Fixture::build(&[(
        "lib.rs",
        &format!("use std::collections::{{BTreeMap, HashMap}};\n{}", src),
    )]);
    f.translated_method("lib.rs", method)
}

const MAPS: &str = "pub struct Lists {\n\
     pub by_name: HashMap<String, Vec<u32>>,\n\
     pub ordered: BTreeMap<String, Vec<u32>>,\n\
     pub counts: HashMap<String, u32>,\n\
     }\n";

/// Every finisher read as a VALUE writes `.value` on the slot. Without it,
/// `entry(k).or_default().push(v)` raised `TypeError: ....push is not a
/// function` the first time a watcher was registered for an entity that had
/// none — five sites in core's `watcherset.rs` alone.
#[test]
fn a_finisher_read_as_a_value_reads_through_the_slot() {
    for (rust, expected) in [
        ("self.by_name.entry(k).or_default().push(v);", ".orDefault(() => []).value.push(v)"),
        ("self.by_name.entry(k).or_insert(Vec::new()).push(v);", ".orInsert([]).value.push(v)"),
        (
            "self.by_name.entry(k).or_insert_with(|| Vec::new()).push(v);",
            ".orInsertWith(() => []).value.push(v)",
        ),
    ] {
        let ts = body(
            &format!("{MAPS}impl Lists {{ pub fn add(&mut self, k: String, v: u32) {{ {rust} }} }}"),
            "add",
        );
        assert!(ts.contains(expected), "wanted `{expected}` in:\n{ts}");
    }
}

/// The one position that wants the slot itself is the `*` that writes through
/// it: `*counts.entry(k).or_insert(0) += 1` has to store into the map.
#[test]
fn a_place_written_through_keeps_the_slot() {
    let ts = body(
        &format!(
            "{MAPS}impl Lists {{ pub fn bump(&mut self, k: String) {{ \
             *self.counts.entry(k).or_insert(0) += 1; }} }}"
        ),
        "bump",
    );
    assert!(ts.contains(".orInsert(0);"), "{}", ts);
    assert!(ts.contains(".value = checkedAdd("), "{}", ts);
    assert!(!ts.contains(".value.value"), "the slot was read twice:\n{ts}");
}

/// `or_default()` reads `V: Default` off the TYPE, which TypeScript cannot, so
/// the port passes the value type's default as a thunk. The value type was read
/// off `hash_map::Entry` alone, so a `BTreeMap` receiver — `btree_map::Entry` is
/// a second declaration, as it is in std — emitted `orDefault()` with no thunk,
/// and `orDefault(undefined)` invokes `undefined` on the first unseen key. Live
/// at core's `ComparisonIndex`, whose `>`, `<`, `>=` and `<=` arms are all
/// `BTreeMap`s.
#[test]
fn a_btree_map_entry_gets_the_value_types_default_too() {
    let ts = body(
        &format!(
            "{MAPS}impl Lists {{ pub fn add(&mut self, k: String, v: u32) {{ \
             self.ordered.entry(k).or_default().push(v); }} }}"
        ),
        "add",
    );
    assert!(ts.contains(".orDefault(() => []).value.push(v)"), "{}", ts);
}

/// A finisher written from its NAME cannot be one: what it writes needs the
/// map's value type, and a receiver the engine could not type says nothing
/// about it. `values.deref_mut().entry(k).or_default()` on core's property
/// write path emitted `orDefault()` with no thunk, which invokes `undefined`
/// the first time a key is unseen (R2).
#[test]
fn a_finisher_written_from_a_name_is_a_hole() {
    for method in ["or_insert(0)", "or_insert_with(|| 0)", "or_default()"] {
        let ts = body(
            &format!(
                "pub struct Loose {{ pub any: u8 }}\n\
                 impl Loose {{ pub fn bump(&self, k: String) {{ \
                 let m = whatever(); m.entry(k).{method}; }} }}"
            ),
            "bump",
        );
        assert!(ts.contains("unsupported("), "wanted a hole for `{method}` in:\n{ts}");
        assert!(!ts.contains("orDefault()"), "the finisher still runs:\n{ts}");
    }
}

/// `impl DerefMut<Target = BTreeMap<K, V>>` binds `Deref`'s `Target`, not one
/// `DerefMut` declares, so the projection was left standing and every call on
/// the map behind it was written from its name. With the bound read through its
/// supertrait the map resolves and the finisher is written from the map's value
/// type — and the `deref_mut()` step itself is the hole, because a bound names
/// no implementor whose dereference the port could write (R2).
#[test]
fn a_bound_answers_a_supertraits_associated_type() {
    let ts = body(
        "use std::ops::DerefMut;\n\
         pub struct Counts;\n\
         impl Counts {\n\
             pub fn bump(mut values: impl DerefMut<Target = BTreeMap<String, u32>>, k: String) {\n\
                 let value = values.deref_mut().entry(k).or_default();\n\
                 *value += 1;\n\
             }\n\
         }\n",
        "bump",
    );
    assert!(ts.contains("unsupported("), "the deref step is not a hole:\n{ts}");
    assert!(!ts.contains(".derefMut()"), "a bound's deref was written as a call:\n{ts}");
    assert!(ts.contains(".orDefault(() => 0)"), "the value type's default is missing:\n{ts}");
}

/// I1: whether a `let` binds the write-through slot is the LOWERING's answer,
/// not a search of the rendered initialiser for `unsupported(`. An argument
/// that is itself a hole leaves the finisher a finisher — the slot is still
/// what it hands back — and a receiver whose value type has no default leaves a
/// hole, whose `.value` would say nothing the hole does not.
#[test]
fn the_slot_is_bound_from_what_was_lowered_not_from_the_text() {
    // The finisher IS lowered; the hole beside it belongs to the argument.
    let ts = body(
        &format!(
            "{MAPS}impl Lists {{ pub fn bump(&mut self, k: String) {{ \
             let slot = self.counts.entry(k).or_insert_with(|| whatever().len()); \
             *slot += 1; }} }}"
        ),
        "bump",
    );
    assert!(ts.contains("const slot = "), "{}", ts);
    assert!(
        !ts.contains("slot.value.value") && ts.contains("slot.value = "),
        "the slot is bound and written through once:\n{ts}"
    );

    // The finisher itself was refused: what the `let` binds is the hole.
    let refused = body(
        "pub struct Lists { pub m: std::collections::HashMap<String, Result<u8, u8>> }\n\
         impl Lists { pub fn take(&mut self, k: String) -> u8 { \
         let slot = self.m.entry(k).or_default(); 0 } }",
        "take",
    );
    assert!(refused.contains("unsupported("), "{}", refused);
    assert!(!refused.contains(").value"), "a hole is not read through:\n{refused}");
}

/// J4: a call the engine REFUSES is replaced by a hole, which throws before
/// anything the call would have consumed reaches a new owner. Counting the
/// receiver and the arguments as moved left the block releasing nothing, so
/// every owned value the refused call named leaked — on the refusal path, which
/// is the one path a reported gap is supposed to make safe.
#[test]
fn a_refused_call_takes_nothing_and_the_block_still_releases_it() {
    let leaked = body(
        "pub struct Key { pub name: String }\n\
         pub struct Val { pub n: u8 }\n\
         pub struct Lists { pub any: u8 }\n\
         impl Lists { pub fn go(&self, k: Key, v: Val) -> u8 { \
         let m = whatever(); m.entry(k).or_insert(v).n } }",
        "go",
    );
    assert!(leaked.contains("unsupported("), "the call is refused:\n{leaked}");
    assert!(leaked.contains("k.drop()"), "the receiver's key is still the block's:\n{leaked}");
    assert!(leaked.contains("v.drop()"), "and so is the argument:\n{leaked}");

    // The other side of the same rule: a finisher the engine CAN write takes
    // both, and the block releases neither.
    let written = body(
        "#[derive(Hash, PartialEq, Eq)]\n\
         pub struct Key { pub name: String }\n\
         pub struct Val { pub n: u8 }\n\
         pub struct Lists { pub m: HashMap<Key, Val> }\n\
         impl Lists { pub fn go(&mut self, k: Key, v: Val) -> u8 { \
         self.m.entry(k).or_insert(v).n } }",
        "go",
    );
    assert!(!written.contains("unsupported("), "{written}");
    assert!(!written.contains(".drop()"), "the entry owns both:\n{written}");
}

/// J4's second half does NOT reproduce at `0dbba0e`: a `DerefMut` bound written
/// as a NAMED generic parameter takes the same route as the argument-position
/// `impl DerefMut`, and all three finishers hole on both. Pinned so the two
/// stay together.
#[test]
fn a_named_deref_mut_bound_takes_the_same_route_as_an_impl_one() {
    let named = body(
        "use std::ops::DerefMut;\n\
         pub struct Val { pub n: u8 }\n\
         pub fn go<M: DerefMut<Target = HashMap<String, Val>>>(values: &mut M, k: String) { \
         values.entry(k).or_insert(Val { n: 0 }); }",
        "go",
    );
    let by_impl = body(
        "use std::ops::DerefMut;\n\
         pub struct Val { pub n: u8 }\n\
         pub fn go(values: &mut impl DerefMut<Target = HashMap<String, Val>>, k: String) { \
         values.entry(k).or_insert(Val { n: 0 }); }",
        "go",
    );
    assert!(named.contains("unsupported("), "{named}");
    assert_eq!(named.trim(), by_impl.trim(), "the two bounds take one route");
}
