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
//! Whether the imports that file writes RESOLVE is a different question, and
//! `tests/import_gate.rs` asks it — this one is only about whether the file
//! parses.
//!
//! The ledger is matched exactly in both directions, like the diagnostics
//! budget: a file that starts being refused fails, and a file that stops being
//! refused fails until its line is taken out. Refresh it deliberately:
//!
//!     cd transpile && UPDATE_PARSE_GATE=1 cargo test --test parse_gate

mod common;

use common::{collect_files_with_ext, crates_in_scope, run_batch, transpile_dir, TempDir};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
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

/// No name a package's index offers is offered by two star exports at once.
///
/// For: Rust keeps `signals::broadcast::ListenerGuard` and
/// `signals::signal::ListenerGuard` apart, because a module is a namespace. The
/// port flattens every public child module into one package surface, and two
/// `export *` lines offering one name export it from NEITHER — JavaScript drops
/// an ambiguous star export silently, and `@ankurah/signals` had no
/// `ListenerGuard` in either spelling. An explicit `export { X } from './m'`
/// shadows the stars, so the index writes one; this test is what says every
/// collision got one.
#[test]
fn no_package_index_offers_a_name_ambiguously() {
    let mut ambiguous: Vec<String> = Vec::new();
    for (package, src) in &crates_in_scope() {
        let out = TempDir::new(&format!("parse-gate-stars-{package}"));
        run_batch(src, out.path(), package);
        let written = collect_files_with_ext(out.path(), Some("ts"));
        let Some(index) = written.get("index.ts") else { continue };
        let mut explicit: BTreeMap<String, ()> = BTreeMap::new();
        let mut stars: Vec<(String, BTreeMap<String, ()>)> = Vec::new();
        for line in index.lines().map(str::trim) {
            if let Some(specifier) = line.strip_prefix("export * from '").and_then(|r| r.split_once('\'')).map(|(s, _)| s) {
                let names = exported_names(&written, specifier, &mut Vec::new());
                stars.push((specifier.to_string(), names));
            } else if line.starts_with("export {") {
                for name in exported_list(line) {
                    explicit.insert(name, ());
                }
            }
        }
        let mut offered: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (specifier, names) in &stars {
            for name in names.keys() {
                offered.entry(name.clone()).or_default().push(specifier.clone());
            }
        }
        for (name, from) in offered {
            if from.len() > 1 && !explicit.contains_key(&name) {
                ambiguous.push(format!("{package}/index.ts: `{name}` from {} and nothing shadows them", from.join(", ")));
            }
        }
    }
    assert!(
        ambiguous.is_empty(),
        "a package index offers {} name(s) from two star exports at once, which exports them from \
         neither:\n  {}",
        ambiguous.len(),
        ambiguous.join("\n  ")
    );
}

/// The names `export * from '<specifier>'` contributes, read out of the emitted
/// files themselves — declarations, named re-exports, and the stars below them.
fn exported_names(
    written: &BTreeMap<String, String>,
    specifier: &str,
    seen: &mut Vec<String>,
) -> BTreeMap<String, ()> {
    let mut out = BTreeMap::new();
    let stem = specifier.trim_start_matches("./").trim_end_matches(".ts");
    let Some((file, text)) = [format!("{stem}.ts"), format!("{stem}/index.ts")]
        .into_iter()
        .find_map(|candidate| written.get_key_value(&candidate))
    else {
        return out;
    };
    if seen.contains(file) {
        return out;
    }
    seen.push(file.clone());
    let dir = file.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
    for line in text.lines().map(str::trim) {
        if let Some(inner) = line.strip_prefix("export * from '").and_then(|r| r.split_once('\'')).map(|(s, _)| s) {
            let joined = join_specifier(&dir, inner);
            for (name, _) in exported_names(written, &joined, seen) {
                out.insert(name, ());
            }
        } else if line.starts_with("export {") {
            for name in exported_list(line) {
                out.insert(name, ());
            }
        } else if let Some(name) = declared_name(line) {
            out.insert(name, ());
        }
    }
    out
}

/// The names an `export { A, B as C }` line offers, as the importer spells them.
fn exported_list(line: &str) -> Vec<String> {
    let Some(body) = line.split_once('{').and_then(|(_, r)| r.split_once('}')).map(|(b, _)| b) else {
        return Vec::new();
    };
    body.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.rsplit(" as ").next().unwrap_or(part).trim().to_string())
        .collect()
}

/// The name an `export class X` / `export function x` / `export type X` line declares.
fn declared_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("export ")?;
    let rest = rest.strip_prefix("declare ").unwrap_or(rest);
    let rest = rest.strip_prefix("abstract ").unwrap_or(rest);
    for keyword in ["class ", "function ", "const ", "let ", "type ", "interface ", "enum "] {
        if let Some(tail) = rest.strip_prefix(keyword) {
            let name: String = tail.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

fn join_specifier(dir: &str, specifier: &str) -> String {
    let mut parts: Vec<&str> = if dir.is_empty() { Vec::new() } else { dir.split('/').collect() };
    for step in specifier.split('/') {
        match step {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Every `catch` the engine writes rethrows an `OwnershipFatal` and an
/// `UnsupportedShape`.
///
/// For: each of those two throws says the run must stop, and a `catch` that
/// turns one into an ordinary `Err` disarms it for everything inside the block
/// — and the emitted JSON readers wrap their whole body in one. The ownership
/// runtime says a value was used after it was dropped, or dropped twice, by
/// throwing an `OwnershipFatal`: that is the whole mechanism by which a port bug
/// stops a run instead of quietly answering nonsense. An R12 hole throws an
/// `UnsupportedShape`: that says the ENGINE has no lowering for a Rust shape,
/// and answering `Err` for it is the loud-into-silent trade R12 exists to
/// refuse. So the rule from `port/ownership.md` is a property of the OUTPUT, not
/// of one emitter: no matter which part of the engine writes a `catch`, the
/// first thing inside it re-throws both.
///
/// The check reads emitted text rather than the engine's format strings so that
/// a `catch` written by some future emitter is caught the day it appears.
#[test]
fn no_emitted_catch_swallows_an_ownership_fatal() {
    let mut swallowing: Vec<String> = Vec::new();
    let mut examined = 0usize;
    for (package, src) in &crates_in_scope() {
        let out = TempDir::new(&format!("parse-gate-catch-{package}"));
        run_batch(src, out.path(), package);
        for (name, text) in &collect_files_with_ext(out.path(), Some("ts")) {
            for (line_no, line) in text.lines().enumerate() {
                if !line.contains("catch (") && !line.contains("catch(") {
                    continue;
                }
                examined += 1;
                let rest: String = text.lines().skip(line_no + 1).take(2).collect::<Vec<_>>().join(" ");
                let same_line = line.split_once("catch").map(|(_, t)| t.to_string()).unwrap_or_default();
                let head = format!("{same_line} {rest}");
                let rethrows = |what: &str| {
                    head.split("throw ").next().is_some_and(|before| {
                        before.contains(&format!("instanceof {}", what))
                    })
                };
                if rethrows("OwnershipFatal") && rethrows("UnsupportedShape") {
                    continue;
                }
                swallowing.push(format!("{package}/{name}:{}: {}", line_no + 1, line.trim()));
            }
        }
    }
    assert!(
        examined > 0,
        "this test found no emitted `catch` at all across ten crates, so it is proving nothing; \
         either the JSON readers stopped being emitted or the scan is looking at the wrong files"
    );
    assert!(
        swallowing.is_empty(),
        "{} emitted `catch` block(s) of {examined} do not rethrow BOTH an OwnershipFatal and \
         an UnsupportedShape, so a double-drop, a use-after-drop, or an R12 hole inside them \
         is answered as an ordinary error (port/ownership.md):\n  {}",
        swallowing.len(),
        swallowing.join("\n  ")
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
