//! Hand-vetted goldens: one small Rust idiom in, the TypeScript it must produce
//! out.
//!
//! Where the corpus snapshots say "output moved", a golden says "this idiom
//! translates to exactly this". Each `goldens/<name>/input.rs` is transpiled as
//! a one-file crate and compared to `goldens/<name>/expected.ts` after trailing
//! whitespace is squared up. There is deliberately no environment variable that
//! rewrites an expected file: a golden changes when a person reads the new
//! output and edits it in.

mod common;

use common::{TempDir, normalize, run_batch, transpile_dir, unified_diff};
use std::path::PathBuf;

#[test]
fn goldens_match() {
    let dir = transpile_dir().join("goldens");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("input.rs").is_file())
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no goldens found under {}", dir.display());

    let mut report = String::new();
    for name in &names {
        report.push_str(&check(&dir.join(name), name));
    }
    assert!(
        report.is_empty(),
        "{} golden(s) checked, output differs:\n\n{report}\n\
         A golden is a decision, not a recording: if the new output is right, edit \
         the expected.ts by hand.",
        names.len()
    );
}

fn check(golden: &PathBuf, name: &str) -> String {
    let out = TempDir::new(&format!("golden-{name}"));
    run_batch(golden, out.path(), name);

    let produced = out.path().join("input.ts");
    if !produced.is_file() {
        return format!("{name}: batch wrote no input.ts (it emitted: {:?})\n", listing(out.path()));
    }
    let actual = normalize(&std::fs::read_to_string(&produced).unwrap());

    let expected_path = golden.join("expected.ts");
    let Ok(expected) = std::fs::read_to_string(&expected_path) else {
        return format!(
            "{name}: no expected.ts. Read the output below, and if it is right, save it there:\n{actual}\n"
        );
    };
    unified_diff(&format!("{name}/expected.ts"), &normalize(&expected), &actual)
}

fn listing(dir: &std::path::Path) -> Vec<String> {
    common::collect_files(dir).into_keys().collect()
}
