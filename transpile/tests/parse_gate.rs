//! Which emitted files a JavaScript engine refuses to load, per crate.
//!
//! For: `tsc --noEmit` is a type checker with error recovery — it reports a
//! duplicate `const`, a `continue` with no loop around it and a stray `>` and
//! then carries on describing the file as if it parsed. A bundler does not: it
//! either produces a module or it refuses, and a module it refuses is a file
//! nothing can import at run time. "Zero parse errors" measured with `tsc` was
//! therefore a statement about recovery, not about whether the port loads; the
//! step-7 review found fourteen emitted core files that `tsc` accepted and bun
//! would not load.
//!
//! So this test asks the second question: `bun build <file> --target=bun
//! --external '*'` over every file `batch` writes for every crate in
//! `transpile.toml`'s `[crates]`. `--external '*'` makes every import somebody
//! else's problem, so what is left is exactly whether this one file parses.
//!
//! The ledger is matched exactly in both directions, like the diagnostics
//! budget: a file that starts being refused fails, and a file that stops being
//! refused fails until its line is taken out. Refresh it deliberately:
//!
//!     cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate

mod common;

use common::{collect_files_with_ext, run_batch, support_tree, transpile_dir, TempDir};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn every_emitted_file_loads() {
    require_bun();
    let crates = crates_in_scope();
    assert!(
        crates.len() >= 10,
        "transpile.toml's [crates] lists {} crates; the port's scope is ten, so either the \
         config lost one or this test is reading the wrong file",
        crates.len()
    );

    let mut refused: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (package, src) in &crates {
        let out = TempDir::new(&format!("parse-gate-{package}"));
        run_batch(src, out.path(), package);
        let mut rows = Vec::new();
        for name in collect_files_with_ext(out.path(), Some("ts")).keys() {
            if let Some(error) = refusal(&out.path().join(name)) {
                rows.push(format!("{name}: {error}"));
            }
        }
        refused.insert(package.clone(), rows);
    }

    let path = transpile_dir().join("tests/parse_gate.toml");
    if std::env::var_os("UPDATE_PARSE_GATE").is_some() {
        std::fs::write(&path, render(&refused)).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
        eprintln!("updated {}", path.display());
        return;
    }

    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nRecord it with:\n    \
             cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate",
            path.display()
        )
    });
    let recorded: toml::Table = text.parse().expect("the parse-gate ledger is not valid TOML");

    let mut moved = String::new();
    for (package, rows) in &refused {
        let listed: Vec<String> = recorded
            .get(package.as_str())
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("refused"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        for row in rows {
            if !listed.contains(row) {
                let _ = writeln!(moved, "\n{package}: a file a JavaScript engine will not load, and the ledger does not have it:\n    {row}");
            }
        }
        for row in &listed {
            if !rows.contains(row) {
                let _ = writeln!(moved, "\n{package}: the ledger records a refusal that is gone. Take the line out:\n    {row}");
            }
        }
    }
    for package in recorded.keys() {
        if !refused.contains_key(package.as_str()) {
            let _ = writeln!(moved, "\nthe ledger names `{package}`, and transpile.toml's [crates] does not");
        }
    }

    let total: usize = refused.values().map(Vec::len).sum();
    assert!(
        moved.is_empty(),
        "which emitted files load has moved ({total} refused across {} crates):\n{moved}\n\
         Fix the emitter, or — once every line above has been read and accepted — refresh with:\n    \
         cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate",
        refused.len()
    );
}

/// The reason a bundler gives for refusing this file, or nothing when it loads.
///
/// Only the first error is kept: the rest are usually the parser trying to
/// recover from the first, and the ledger is a list of defects rather than a
/// transcript.
fn refusal(file: &Path) -> Option<String> {
    let output = Command::new("bun")
        .arg("build")
        .arg(file)
        .arg("--target=bun")
        .arg("--external")
        .arg("*")
        .output()
        .unwrap_or_else(|e| panic!("cannot run bun build on {}: {e}", file.display()));
    if output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stderr).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    let first = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("error:"))
        .unwrap_or("bun build failed and said nothing this test could read");
    Some(first.trim_start_matches("error:").trim().to_string())
}

/// Every crate the port is in scope for, as its package name and the directory
/// `batch` is pointed at.
///
/// The list comes from `transpile.toml` rather than from a table here, so a
/// crate entering or leaving the port's scope moves this test with it. Where
/// each crate's sources sit comes from the crate's own `Cargo.toml`, the same
/// way the engine's sibling loader finds them.
fn crates_in_scope() -> Vec<(String, PathBuf)> {
    let config = transpile_dir().join("transpile.toml");
    let text = std::fs::read_to_string(&config).unwrap_or_else(|e| panic!("cannot read {}: {e}", config.display()));
    let table: toml::Table = text.parse().expect("transpile.toml is not valid TOML");
    let crates = table
        .get("crates")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("transpile.toml has no [crates] table"));

    let manifests = manifests_under(&support_tree());
    let mut out = Vec::new();
    for (crate_name, package) in crates {
        let package = package.as_str().unwrap_or_else(|| panic!("[crates] {crate_name} is not a string"));
        let dir = manifests.get(crate_name).unwrap_or_else(|| {
            panic!("no Cargo.toml under {} declares the package `{crate_name}`", support_tree().display())
        });
        let src = dir.join("src");
        assert!(src.is_dir(), "`{crate_name}` has no src/ at {}", src.display());
        out.push((package.to_string(), src));
    }
    out.sort();
    out
}

/// Every Cargo package under the corpus, by name, and the directory it lives in.
fn manifests_under(root: &Path) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    walk_manifests(root, &mut out);
    out
}

fn walk_manifests(dir: &Path, out: &mut BTreeMap<String, PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        if path.is_dir() {
            // `target/` holds thousands of vendored manifests and no corpus crate.
            if name == "target" || name == "node_modules" || name.starts_with('.') {
                continue;
            }
            walk_manifests(&path, out);
        } else if name == "Cargo.toml" {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Ok(manifest) = text.parse::<toml::Table>() else { continue };
            if let Some(package) = manifest.get("package").and_then(|v| v.as_table()).and_then(|t| t.get("name")).and_then(|v| v.as_str()) {
                out.insert(package.to_string(), path.parent().unwrap().to_path_buf());
            }
        }
    }
}

fn render(refused: &BTreeMap<String, Vec<String>>) -> String {
    let total: usize = refused.values().map(Vec::len).sum();
    let mut out = String::new();
    out.push_str(
        "# Emitted files a JavaScript engine refuses to load, written by\n\
         # transpile/tests/parse_gate.rs. Each line is one file `bun build` will not\n\
         # produce a module for, with the first error it gave. `tsc` recovers from\n\
         # every one of these and reports the file as parsed, which is why this\n\
         # ledger exists beside the diagnostics budget. Matched EXACTLY in both\n\
         # directions: a new refusal fails, and a refusal that is fixed fails until\n\
         # its line is taken out. Generated: do not hand-edit. Refresh with:\n\
         #     cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate\n",
    );
    let _ = writeln!(out, "#\n# Total refused: {total}\n");
    for (package, rows) in refused {
        let _ = writeln!(out, "[{package}]");
        if rows.is_empty() {
            out.push_str("refused = []\n\n");
            continue;
        }
        out.push_str("refused = [\n");
        for row in rows {
            let _ = writeln!(out, "  {},", toml_string(row));
        }
        out.push_str("]\n\n");
    }
    out
}

fn toml_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn require_bun() {
    match Command::new("bun").arg("--version").output() {
        Ok(probe) if probe.status.success() => {}
        Ok(probe) => panic!("`bun --version` failed ({}):\n{}", probe.status, String::from_utf8_lossy(&probe.stderr)),
        Err(e) => panic!(
            "cannot run `bun`: {e}. This test asks a JavaScript engine whether the emitted files \
             load, so bun has to be on PATH."
        ),
    }
}

/// Every relative module specifier an emitted file writes names a file the same
/// batch produced.
///
/// For: `bun build --external '*'` makes every import somebody else's problem,
/// so it cannot see a specifier that resolves to nothing. The facade's emitted
/// index was thirty-one lines of `export { X } from './ankurah_core'` against a
/// directory holding two files — 29 `TS2307`s, which was every own-file error
/// that package had — because the registry keeps a SIBLING CRATE's root among a
/// module's children and `public_reexports` read one as the other. Nothing in
/// the harness asked whether the port could be imported at all.
#[test]
fn every_relative_import_names_a_file_the_batch_wrote() {
    let crates = crates_in_scope();
    // A `[[provided]]` module is TypeScript somebody wrote by hand; the batch
    // does not write it and the package supplies it, so an import naming one
    // resolves in the package and not here.
    let provided = provided_modules();
    let mut missing: Vec<String> = Vec::new();
    for (package, src) in &crates {
        let out = TempDir::new(&format!("parse-gate-imports-{package}"));
        run_batch(src, out.path(), package);
        let written = collect_files_with_ext(out.path(), Some("ts"));
        for (name, text) in &written {
            for specifier in relative_specifiers(text) {
                if resolves(&written, name, &specifier, &provided) {
                    continue;
                }
                missing.push(format!("{package}/{name}: `{specifier}` names no emitted file"));
            }
        }
    }
    // A ledger, matched exactly in both directions, like the refusals above:
    // an import that starts naming nothing fails, and one that stops fails
    // until its line comes out. The facade's 29 were every own-file error that
    // package had; what is left is a list of modules the port owes.
    let path = transpile_dir().join("tests/import_gate.toml");
    missing.sort();
    if std::env::var_os("UPDATE_PARSE_GATE").is_some() {
        let mut text = String::from(
            "# Emitted imports that name a module `batch` does not write and\n             # `transpile.toml` does not declare as `[[provided]]`. Written by\n             # transpile/tests/parse_gate.rs. Matched EXACTLY in both directions.\n             # Generated: do not hand-edit. Refresh with:\n             #     cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate\n\n             unresolved = [\n",
        );
        for row in &missing {
            let _ = writeln!(text, "  {:?},", row);
        }
        text.push_str("]\n");
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
        eprintln!("updated {}", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nRecord it with:\n                 cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate",
            path.display()
        )
    });
    let recorded: toml::Table = text.parse().expect("the import ledger is not valid TOML");
    let listed: Vec<String> = recorded
        .get("unresolved")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let mut moved = String::new();
    for row in &missing {
        if !listed.contains(row) {
            let _ = writeln!(moved, "  new: {row}");
        }
    }
    for row in &listed {
        if !missing.contains(row) {
            let _ = writeln!(moved, "  gone, take the line out: {row}");
        }
    }
    assert!(
        moved.is_empty(),
        "which emitted imports name nothing has moved ({} unresolved):\n{moved}\n         Fix the emitter, or — once every line above has been read — refresh with:\n             cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate",
        missing.len()
    );
}

/// Every `'./x'` or `'../x'` an import or export line names.
fn relative_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("import ") && !trimmed.starts_with("export ") {
            continue;
        }
        let Some(open) = line.rfind('\'') else { continue };
        let before = &line[..open];
        let Some(start) = before.rfind('\'') else { continue };
        let specifier = &line[start + 1..open];
        if specifier.starts_with("./") || specifier.starts_with("../") {
            out.push(specifier.to_string());
        }
    }
    out
}

/// Does this specifier, written in `from`, name one of the files the batch
/// wrote? A directory resolves through its `index.ts`, which is what the port
/// writes for a module with children.
fn resolves(
    written: &BTreeMap<String, String>,
    from: &str,
    specifier: &str,
    provided: &[String],
) -> bool {
    let mut parts: Vec<String> = Vec::new();
    if let Some(dir) = Path::new(from).parent() {
        for part in dir.components() {
            parts.push(part.as_os_str().to_string_lossy().to_string());
        }
    }
    for step in specifier.split('/') {
        match step {
            "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other.to_string()),
        }
    }
    let joined = parts.join("/");
    let joined = joined.trim_end_matches(".ts").to_string();
    written.contains_key(&format!("{joined}.ts"))
        || written.contains_key(&format!("{joined}/index.ts"))
        || provided.iter().any(|m| *m == joined || *m == format!("{joined}/index"))
}

/// Every `[[provided]]` and `[[extra_exports]]` TypeScript module the config
/// names, as the path an import would write for it.
fn provided_modules() -> Vec<String> {
    let path = transpile_dir().join("transpile.toml");
    let text = std::fs::read_to_string(&path).expect("transpile.toml is readable");
    let table: toml::Table = text.parse().expect("transpile.toml is valid TOML");
    let mut out = Vec::new();
    for key in ["provided", "extra_exports"] {
        let Some(entries) = table.get(key).and_then(|v| v.as_array()) else {
            continue;
        };
        for entry in entries {
            if let Some(module) = entry.get("module").and_then(|v| v.as_str()) {
                // `util/index` is imported as `./util`.
                out.push(module.to_string());
                if let Some(dir) = module.strip_suffix("/index") {
                    out.push(dir.to_string());
                }
                // A hand-written file is `<module>.provided.ts` on disk and
                // `./<module>` in an import.
                out.push(module.replace(".provided", ""));
            }
        }
    }
    out
}
