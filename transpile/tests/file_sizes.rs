//! How long the transpiler's own source files are, ratcheted down.
//!
//! For: the project's rule is that a file stays under about 600 lines and is
//! split before it grows past it. The rule had no mechanism, so it went
//! backwards twice — 21 files over the line, then 23, then 26 — and every
//! review since has had to count by hand. A rule nobody measures is a
//! preference.
//!
//! A test that simply failed at 600 would be red on 26 files today, and a red
//! test in the harness stops saying anything at all. So this one is a RATCHET:
//! it records what each over-long file measures now and fails when one grows,
//! or when a new file crosses the line. Shrinking a file fails too, until its
//! recorded number comes down with it — which is what keeps the ledger honest
//! about where the work has actually been done.
//!
//!     cd transpile && UPDATE_FILE_SIZES=1 cargo test --test file_sizes

mod common;

use common::transpile_dir;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The line the rule draws. A file at or under it is not recorded at all, so
/// the ledger names exactly the files that owe a split.
const LIMIT: usize = 600;

#[test]
fn no_transpiler_source_file_grows_past_what_is_recorded() {
    let root = transpile_dir();
    let mut measured: BTreeMap<String, usize> = BTreeMap::new();
    for path in rust_files(&root) {
        let lines = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
            .lines()
            .count();
        if lines <= LIMIT {
            continue;
        }
        let name = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        measured.insert(name, lines);
    }

    let ledger = root.join("tests/file_sizes.toml");
    if std::env::var_os("UPDATE_FILE_SIZES").is_some() {
        std::fs::write(&ledger, render(&measured))
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", ledger.display()));
        eprintln!("updated {}", ledger.display());
        return;
    }

    let text = std::fs::read_to_string(&ledger).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nRecord it with:\n    \
             cd transpile && UPDATE_FILE_SIZES=1 cargo test --test file_sizes",
            ledger.display()
        )
    });
    let recorded: toml::Table = text.parse().expect("the file-size ledger is not valid TOML");
    let recorded: BTreeMap<String, usize> = recorded
        .get("over")
        .and_then(|v| v.as_table())
        .map(|t| {
            t.iter()
                .filter_map(|(k, v)| v.as_integer().map(|n| (k.clone(), n as usize)))
                .collect()
        })
        .unwrap_or_default();

    let mut moved = String::new();
    for (name, lines) in &measured {
        match recorded.get(name) {
            Some(was) if lines > was => {
                let _ = writeln!(
                    moved,
                    "\n{name} grew from {was} to {lines} lines. Split it, or move the work into a \
                     file that is not already over {LIMIT}."
                );
            }
            Some(was) if lines < was => {
                let _ = writeln!(
                    moved,
                    "\n{name} is {lines} lines and the ledger says {was}. Bring the number down."
                );
            }
            None => {
                let _ = writeln!(
                    moved,
                    "\n{name} is {lines} lines, which is over {LIMIT}, and the ledger does not \
                     name it. Split it before it grows."
                );
            }
            _ => {}
        }
    }
    for (name, was) in &recorded {
        if !measured.contains_key(name) {
            let _ = writeln!(
                moved,
                "\n{name} is under {LIMIT} now (the ledger says {was}). Take its line out."
            );
        }
    }

    assert!(
        moved.is_empty(),
        "the transpiler's file sizes have moved ({} files over {LIMIT} lines):\n{moved}\n\
         Once every line above has been read and accepted, refresh with:\n    \
         cd transpile && UPDATE_FILE_SIZES=1 cargo test --test file_sizes",
        measured.len()
    );
}

/// Every `.rs` file the transpiler's own tree holds, tests included: a test
/// module that outgrows the rule is as hard to read as any other file.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in ["src", "tests"] {
        walk(&root.join(dir), &mut out);
    }
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn render(measured: &BTreeMap<String, usize>) -> String {
    let mut out = String::from(HEADER);
    out.push_str("[over]\n");
    for (name, lines) in measured {
        let _ = writeln!(out, "{} = {lines}", toml::Value::String(name.clone()));
    }
    out
}

const HEADER: &str = "\
# Transpiler source files over 600 lines, written by
# transpile/tests/file_sizes.rs. The project's rule is that a file stays under
# about 600 lines and is split before it grows past one; this ledger is the
# ratchet that keeps the rule from going backwards while the files that are
# already over it come down.
#
# Matched EXACTLY: a file that grows fails, a file that shrinks fails until its
# number comes down with it, a new file over the line fails, and a file that
# drops under it fails until its line is taken out.
#
# Generated: do not hand-edit. Refresh with:
#     cd transpile && UPDATE_FILE_SIZES=1 cargo test --test file_sizes

";
