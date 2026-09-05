//! Every static the emitted code CALLS is one something declares.
//!
//! For: the engine writes `T.fromJson(v)` for a field whose type reads JSON, and
//! it decides that from the registry. A type whose TypeScript a person wrote is
//! not in the registry's gift — its members are whatever that person wrote — so
//! "this type is hand-written" was never evidence that the file declares a
//! `fromJson`. Read as evidence, it put `Attested.fromJson` in three emitted
//! proto call sites where `auth.provided.ts` declares no such static, and the
//! JSON readers that reached one raised a `TypeError` at run time.
//!
//! So the rule is a property of the OUTPUT: for every `X.fromJson(` the emitter
//! writes, either an emitted file declares `static fromJson` on `X`, or the
//! `[provided_impls]` entry for `X` says `reads_json = true` AND the file it
//! names really declares one. The last clause is what keeps the config honest:
//! a `reads_json = true` beside a file that has no such static fails here.

mod common;

use common::{collect_files_with_ext, run_batch, support_tree, transpile_dir, TempDir};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[test]
fn every_from_json_call_names_a_declared_static() {
    let declared_by_hand = provided_types_declaring_from_json();
    let mut missing: Vec<String> = Vec::new();
    let mut called = 0usize;

    for (package, src) in crates_in_scope() {
        let out = TempDir::new(&format!("declared-members-{package}"));
        run_batch(&src, out.path(), &package);
        let files = collect_files_with_ext(out.path(), Some("ts"));

        // Every class this crate's own output declares the static on.
        let mut emitted: BTreeSet<String> = BTreeSet::new();
        for text in files.values() {
            let mut class = String::new();
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("export class ") {
                    class = rest
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("")
                        .to_string();
                }
                if line.trim_start().starts_with("static fromJson(") && !class.is_empty() {
                    emitted.insert(class.clone());
                }
            }
        }

        for (name, text) in &files {
            for (line_no, line) in text.lines().enumerate() {
                for class in from_json_receivers(line) {
                    called += 1;
                    if emitted.contains(&class) || declared_by_hand.contains(&class) {
                        continue;
                    }
                    missing.push(format!(
                        "{package}/{name}:{}: {} — {}",
                        line_no + 1,
                        line.trim(),
                        class
                    ));
                }
            }
        }
    }

    assert!(
        called > 0,
        "this test found no `fromJson` call at all across ten crates, so it is proving nothing; \
         either the JSON readers stopped being emitted or the scan reads the wrong files"
    );
    assert!(
        missing.is_empty(),
        "{} emitted call(s) name a `fromJson` nothing declares — a TypeError the first time the \
         reader runs:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}

/// The classes `[provided_impls]` says have a hand-written `fromJson`, verified
/// against the file each entry names.
///
/// Reading the file is the point: the entry is a claim about TypeScript the
/// engine never sees, and an unverified claim is the same defect one indirection
/// further on.
fn provided_types_declaring_from_json() -> BTreeSet<String> {
    let table = config_table();
    let provided = table
        .get("provided_impls")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("transpile.toml has no [provided_impls] table"));
    let crates = table
        .get("crates")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("transpile.toml has no [crates] table"));

    let mut out = BTreeSet::new();
    let mut checked = 0usize;
    for (fqn, entry) in provided {
        let entry = entry.as_table().unwrap_or_else(|| panic!("[provided_impls] {fqn} is not a table"));
        if !entry.get("reads_json").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        let path = entry
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("[provided_impls] {fqn} has no `path`"));
        let class = fqn.rsplit("::").next().unwrap_or(fqn).to_string();
        let file = provided_file(crates, fqn, path);
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("[provided_impls] {fqn} names {}, which cannot be read: {e}", file.display()));
        assert!(
            text.contains("static fromJson("),
            "[provided_impls] {fqn} says `reads_json = true`, but {} declares no \
             `static fromJson`. Either the file lost it or the entry is wrong; emitted code \
             calls that static.",
            file.display()
        );
        checked += 1;
        out.insert(class);
    }
    assert!(
        checked > 0,
        "no [provided_impls] entry says `reads_json = true`, so this check is proving nothing"
    );
    out
}

/// Where the hand-written file for a `[provided_impls]` entry lives: the
/// package's `src/`, beside the emitted output it is re-exported from.
fn provided_file(crates: &toml::Table, fqn: &str, path: &str) -> PathBuf {
    let rust_crate = fqn.split("::").next().unwrap_or("").replace('_', "-");
    let package = crates
        .get(&rust_crate)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("[crates] does not name `{rust_crate}`, the crate of {fqn}"));
    checkout_root().join("packages").join(package).join("src").join(format!("{path}.ts"))
}

/// The checkout the engine and the packages share: `transpile/` sits in it.
fn checkout_root() -> PathBuf {
    transpile_dir()
        .parent()
        .unwrap_or_else(|| panic!("transpile/ has no parent directory"))
        .to_path_buf()
}

fn config_table() -> toml::Table {
    let config = transpile_dir().join("transpile.toml");
    std::fs::read_to_string(&config)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", config.display()))
        .parse()
        .expect("transpile.toml is not valid TOML")
}

/// The class names a line calls `fromJson` on, as `X` in `X.fromJson(`.
fn from_json_receivers(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (at, _) in line.match_indices(".fromJson(") {
        let before = &line[..at];
        let name: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        // `this.fromJson(` and a lower-case receiver are not a class static.
        if name.chars().next().is_some_and(|c| c.is_uppercase()) {
            out.push(name);
        }
    }
    out
}

/// The ten packages, with the corpus directory each is transpiled from.
fn crates_in_scope() -> Vec<(String, PathBuf)> {
    let table = config_table();
    let crates = table
        .get("crates")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("transpile.toml has no [crates] table"));
    let manifests = manifests_under(&support_tree());
    let mut out = Vec::new();
    for (crate_name, package) in crates {
        let package = package.as_str().unwrap_or_else(|| panic!("[crates] {crate_name} is not a string"));
        let dir = manifests
            .get(crate_name)
            .unwrap_or_else(|| panic!("no Cargo.toml under the corpus declares `{crate_name}`"));
        out.push((package.to_string(), dir.join("src")));
    }
    out.sort();
    out
}

fn manifests_under(root: &Path) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    walk(root, &mut out);
    out
}

fn walk(dir: &Path, out: &mut BTreeMap<String, PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target" || n == "node_modules") {
                continue;
            }
            walk(&path, out);
        } else if path.file_name().is_some_and(|n| n == "Cargo.toml") {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Ok(manifest) = text.parse::<toml::Table>() else { continue };
            if let Some(name) = manifest
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                out.insert(name.to_string(), path.parent().unwrap().to_path_buf());
            }
        }
    }
}
