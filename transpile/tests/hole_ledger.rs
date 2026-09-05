//! Every R12 hole the engine computes, recorded where it matters: in the file,
//! by name.
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
//! The ledger used to be a COUNT per crate, and a count cannot tell a swap from
//! a standstill: remove one of core's or-pattern holes, add an unrelated new
//! one, and six is still six. So each hole is recorded by an identity that
//! survives an edit somewhere else — the emitted file it lands in and the
//! refusal it prints — and the whole table is matched. The per-crate totals are
//! still written, as a summary a reader can scan; they are not the assertion.
//!
//! ```
//! cd transpile && cargo test --test hole_ledger
//! cd transpile && UPDATE_HOLE_LEDGER=1 cargo test --test hole_ledger
//! ```

mod common;

use common::{collect_files_with_ext, crates_in_scope, run_batch, transpile_dir, TempDir};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Where each `unsupported(..)` call lands and what it says. Written by this
/// test under `UPDATE_HOLE_LEDGER=1`. Generated: read the change before
/// recording it.
const LEDGER: &str = include_str!("hole_ledger.toml");

/// One recorded hole: the crate and emitted file it is written into, and the
/// refusal it prints. The count is how many calls in that file print it — the
/// or-pattern refusal is one semantic site written as a test and two bindings.
type Holes = BTreeMap<(String, String, String), usize>;

#[test]
fn every_computed_hole_reaches_the_emitted_file() {
    let found = emit_and_collect();

    if std::env::var_os("UPDATE_HOLE_LEDGER").is_some() {
        std::fs::write(transpile_dir().join("tests/hole_ledger.toml"), render(&found))
            .expect("cannot write the hole ledger");
        eprintln!("updated tests/hole_ledger.toml");
        return;
    }

    let recorded = parse(LEDGER);
    let mut moved: Vec<String> = Vec::new();
    for (key, count) in &found {
        match recorded.get(key) {
            None => moved.push(format!("  APPEARED {}", describe(key, *count))),
            Some(was) if was != count => {
                moved.push(format!("  {} → {} calls: {}", was, count, describe(key, *count)));
            }
            Some(_) => {}
        }
    }
    for (key, count) in &recorded {
        if !found.contains_key(key) {
            moved.push(format!("  GONE     {}", describe(key, *count)));
        }
    }

    assert!(
        moved.is_empty(),
        "the holes in the emitted output moved:\n{}\n\nA hole is the engine refusing a shape it \
         cannot write, and it only refuses anything if it reaches the file. One that APPEARED is \
         a shape the engine stopped lowering; one that is GONE is either a shape it started \
         lowering or a refusal that was lost on the way out. Read the change, then record it \
         with:\n    cd transpile && UPDATE_HOLE_LEDGER=1 cargo test --test hole_ledger",
        moved.join("\n")
    );
}

/// Re-emit every crate in scope and read the holes out of what it wrote.
fn emit_and_collect() -> Holes {
    let mut found: Holes = BTreeMap::new();
    for (package, src) in crates_in_scope() {
        let out = TempDir::new(&format!("hole-ledger-{package}"));
        run_batch(&src, out.path(), &package);
        for (file, text) in collect_files_with_ext(out.path(), Some("ts")) {
            for message in refusals(&text) {
                *found.entry((package.clone(), file.clone(), message)).or_insert(0) += 1;
            }
        }
    }
    found
}

/// Every `unsupported('..')` call's message, in the order it is written.
///
/// The argument is a single-quoted TypeScript string the emitter escaped, so
/// the message ends at the first quote that is not escaped. Reading it that way
/// rather than by counting occurrences is the whole point of this ledger.
fn refusals(text: &str) -> Vec<String> {
    const OPEN: &str = "unsupported('";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        rest = &rest[at + OPEN.len()..];
        let mut message = String::new();
        let mut chars = rest.char_indices();
        let mut end = rest.len();
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' => {
                    if let Some((_, escaped)) = chars.next() {
                        message.push(escaped);
                    }
                }
                '\'' => {
                    end = i + 1;
                    break;
                }
                other => message.push(other),
            }
        }
        rest = &rest[end..];
        out.push(message);
    }
    out
}

fn describe(key: &(String, String, String), count: usize) -> String {
    format!("{}/{} ×{}: {}", key.0, key.1, count, key.2)
}

/// The ledger as TOML: one `[[hole]]` per identity, plus the per-crate totals.
fn render(found: &Holes) -> String {
    let mut text = String::from(
        "# Every R12 hole in the emitted output, by the identity that survives an\n\
         # edit somewhere else: the crate, the emitted file, and the refusal the\n\
         # call prints. `calls` is how many calls in that file print it — the\n\
         # or-pattern refusal is one semantic site written as a test and two\n\
         # bindings. Written by transpile/tests/hole_ledger.rs and matched\n\
         # EXACTLY: a hole that appears has to be recorded, and one that\n\
         # disappears has to be explained.\n\
         #\n\
         # Generated: do not hand-edit. Refresh with:\n\
         #     cd transpile && UPDATE_HOLE_LEDGER=1 cargo test --test hole_ledger\n",
    );

    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    for ((package, _, _), count) in found {
        *totals.entry(package.as_str()).or_insert(0) += count;
    }
    text.push_str("\n# A summary, for a reader. The assertion is the table below it.\n[summary]\n");
    for (package, _) in crates_in_scope() {
        let _ = writeln!(text, "\"{}\" = {}", package, totals.get(package.as_str()).unwrap_or(&0));
    }

    for ((package, file, message), count) in found {
        let _ = write!(
            text,
            "\n[[hole]]\ncrate = \"{}\"\nfile = \"{}\"\ncalls = {}\nrefusal = \"{}\"\n",
            package,
            file,
            count,
            message.replace('\\', "\\\\").replace('"', "\\\"")
        );
    }
    text
}

fn parse(text: &str) -> Holes {
    let table: toml::Table = text.parse().expect("hole_ledger.toml is not valid TOML");
    let mut out: Holes = BTreeMap::new();
    let Some(holes) = table.get("hole") else { return out };
    let holes = holes.as_array().expect("`hole` in the ledger is not an array of tables");
    for hole in holes {
        let field = |name: &str| -> String {
            hole.get(name)
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("a [[hole]] in the ledger has no `{name}`"))
                .to_string()
        };
        let calls = hole
            .get("calls")
            .and_then(|v| v.as_integer())
            .unwrap_or_else(|| panic!("a [[hole]] in the ledger has no `calls`"));
        out.insert((field("crate"), field("file"), field("refusal")), calls as usize);
    }
    out
}
