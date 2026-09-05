//! Every R12 hole the engine computes, counted where it matters: in the file.
//!
//! For: a hole is the engine refusing to write a shape it has no lowering for,
//! and the refusal only does its job if it reaches the emitted TypeScript. A
//! hole computed during body translation and then discarded by a later stage is
//! not a hole — it is a diagnostic filed about output that answers something
//! else. That happened: `Property::from_value` for `Json` carried a hole, its
//! emitted name collided with `From<serde_json::Value>`'s, the emitter dropped
//! the whole method, and `Json.fromValue` answered `new Json(value)` where Rust
//! answers `Err(PropertyError::Missing)`. The diagnostic was filed; nothing
//! checked that the hole survived.
//!
//! So the count is a ledger, matched EXACTLY in both directions like the
//! diagnostics budget: a hole that appears has to be recorded, and a hole that
//! disappears has to be explained — either the shape is now lowered, or the
//! refusal was lost.
//!
//! ```
//! cd transpile && cargo test --test hole_ledger
//! cd transpile && UPDATE_HOLE_LEDGER=1 cargo test --test hole_ledger
//! ```

mod common;

use common::{collect_files_with_ext, crates_in_scope, run_batch, transpile_dir, TempDir};

/// How many `unsupported(..)` calls each crate's emitted output carries.
///
/// Written by this test under `UPDATE_HOLE_LEDGER=1`. Generated: read the
/// change before recording it.
const LEDGER: &str = include_str!("hole_ledger.toml");

#[test]
fn every_computed_hole_reaches_the_emitted_file() {
    let recorded: toml::Table = LEDGER.parse().expect("hole_ledger.toml is not valid TOML");
    let mut found: Vec<(String, usize)> = Vec::new();
    for (package, src) in crates_in_scope() {
        let out = TempDir::new(&format!("hole-ledger-{package}"));
        run_batch(&src, out.path(), &package);
        let holes = collect_files_with_ext(out.path(), Some("ts"))
            .values()
            .map(|text| text.matches("unsupported(").count())
            .sum();
        found.push((package, holes));
    }

    if std::env::var_os("UPDATE_HOLE_LEDGER").is_some() {
        let mut text = String::from(
            "# How many R12 holes each crate's emitted output carries, written by\n\
             # transpile/tests/hole_ledger.rs. Matched EXACTLY in both directions: a\n\
             # hole that appears has to be recorded, and one that disappears has to be\n\
             # explained. Generated: do not hand-edit. Refresh with:\n\
             #     cd transpile && UPDATE_HOLE_LEDGER=1 cargo test --test hole_ledger\n\n",
        );
        for (package, holes) in &found {
            text.push_str(&format!("\"{}\" = {}\n", package, holes));
        }
        std::fs::write(transpile_dir().join("tests/hole_ledger.toml"), text)
            .expect("cannot write the hole ledger");
        return;
    }

    let mut moved: Vec<String> = Vec::new();
    for (package, holes) in &found {
        let was = recorded
            .get(package.as_str())
            .and_then(|v| v.as_integer())
            .unwrap_or_else(|| panic!("the hole ledger has no line for `{package}`"));
        if was as usize != *holes {
            moved.push(format!("  {package}: {holes} hole(s) in the output, recorded {was}"));
        }
    }
    assert!(
        moved.is_empty(),
        "the holes in the emitted output moved:\n{}\n\nA hole is the engine refusing a shape it \
         cannot write, and it only refuses anything if it reaches the file. A RISE is a shape the \
         engine stopped lowering; a FALL is either one it started lowering or a refusal that was \
         lost on the way out. Read the change, then record it with:\n    cd transpile && \
         UPDATE_HOLE_LEDGER=1 cargo test --test hole_ledger",
        moved.join("\n")
    );
}
