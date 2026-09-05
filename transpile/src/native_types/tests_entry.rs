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
