//! Regression snapshots for whole-crate transpiler output (spec section 6.4).
//!
//! The stored trees under `tests/snapshots/<crate>` are what `batch` produced at
//! the commit the harness was built on. Re-running `batch` and diffing tells you
//! that an engine change moved output, and exactly where; whether the move is a
//! fix or a regression is a judgement call for the reviewer, never for the test.
//!
//! Refresh a snapshot only when the new output has been read and accepted:
//!
//!     cd transpile && UPDATE_SNAPSHOTS=1 cargo test --test corpus_snapshots

mod common;

use common::{TempDir, collect_files, normalize, run_batch, support_tree, transpile_dir, unified_diff};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn proto_snapshot() { check("proto") }

#[test]
fn ankql_snapshot() { check("ankql") }

#[test]
fn signals_snapshot() { check("signals") }

fn snapshot_dir(crate_name: &str) -> PathBuf { transpile_dir().join("tests/snapshots").join(crate_name) }

fn check(crate_name: &str) {
    let out = TempDir::new(crate_name);
    run_batch(&support_tree().join(crate_name).join("src"), out.path(), crate_name);

    let actual: Vec<(String, String)> =
        collect_files(out.path()).into_iter().map(|(k, v)| (k, normalize(&v))).collect();
    let dir = snapshot_dir(crate_name);

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        write_snapshot(&dir, &actual);
        return;
    }

    assert!(
        dir.is_dir(),
        "no snapshot for {crate_name} at {}; create it with UPDATE_SNAPSHOTS=1 cargo test",
        dir.display()
    );
    let expected: Vec<(String, String)> =
        collect_files(&dir).into_iter().map(|(k, v)| (k, normalize(&v))).collect();

    let expected_names: BTreeSet<&str> = expected.iter().map(|(k, _)| k.as_str()).collect();
    let actual_names: BTreeSet<&str> = actual.iter().map(|(k, _)| k.as_str()).collect();

    let mut report = String::new();
    for name in expected_names.difference(&actual_names) {
        report.push_str(&format!("MISSING (snapshot has it, batch no longer emits it): {name}\n"));
    }
    for name in actual_names.difference(&expected_names) {
        report.push_str(&format!("NEW (batch emits it, snapshot does not have it): {name}\n"));
    }
    for (name, actual_text) in &actual {
        if let Some((_, expected_text)) = expected.iter().find(|(k, _)| k == name) {
            report.push_str(&unified_diff(name, expected_text, actual_text));
        }
    }

    assert!(
        report.is_empty(),
        "transpiler output for {crate_name} moved away from tests/snapshots/{crate_name}:\n\n{report}\n\
         If every change above is a reviewed improvement, refresh with:\n    \
         cd transpile && UPDATE_SNAPSHOTS=1 cargo test --test corpus_snapshots"
    );
}

fn write_snapshot(dir: &PathBuf, files: &[(String, String)]) {
    // Drop files the transpiler no longer emits, so a refreshed snapshot is the
    // whole truth about what batch produces rather than an accreting pile.
    if dir.is_dir() {
        for name in collect_files(dir).keys() {
            std::fs::remove_file(dir.join(name)).ok();
        }
    }
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
    eprintln!("updated snapshot {} ({} files)", dir.display(), files.len());
}
