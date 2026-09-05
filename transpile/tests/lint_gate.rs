//! Which names emitted code uses without declaring them, per crate — and which
//! emitted lines call one function twice.
//!
//! For: three of the defects the reviews found this cycle were bare undeclared
//! identifiers in emitted TypeScript — `AtomicBool.new(..)` where nothing
//! exports an `AtomicBool`, `oRIGIN.drop()` naming a binding the arm no longer
//! declares, `Ulid` in ankql's `ast.ts`. Each is a `ReferenceError` on the line
//! that runs it, and each was invisible: `tsc` reports them among thousands of
//! semantic errors, and the parse gate cannot see them because a file that
//! names something undeclared still parses.
//!
//! ESLint's `no-undef` asks exactly that question, and it asks it of a scope
//! rather than a type checker, so the answer is short and readable. Four more
//! rules ride along because each names a shape the emitter has produced before
//! and none of them is ever right: a duplicate key in an object literal
//! (`no-dupe-keys`), a statement after a `return` (`no-unreachable`), `x = x`
//! (`no-self-assign`), and `if (false)` (`no-constant-condition`, loops
//! excepted because `while (true)` is how every emitted `loop` is written).
//!
//! The second check in the same target is the double-receiver defect: an
//! emitted expression that calls one function twice, which happens when a
//! lowering reads its subject once for a test and again to hand it on. Rust
//! reads it once, so the second call performs whatever the first one did all
//! over again — `subscriptions.remove(id)` removed the entry, discarded it, and
//! removed nothing the second time. It is approximated by scanning each emitted
//! line for a repeated `identifier(..)` call, which is what the fix-pass
//! self-reviews have been doing by hand.
//!
//! Both are ledgers, matched EXACTLY in both directions like the parse gate and
//! the diagnostics budget: a new occurrence fails, and one that is gone fails
//! until its line is taken out. Refresh deliberately:
//!
//!     cd transpile && UPDATE_LINT_GATE=1 cargo test --test lint_gate

mod common;

use common::{collect_files_with_ext, crates_in_scope, run_batch, transpile_dir, TempDir};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

/// The globals an emitted file may name without importing: the JavaScript
/// language's own, the host objects the port's targets supply, and the
/// TypeScript utility types that stand in a type position.
///
/// Anything else an emitted file names has to come from an `import`, and a name
/// that comes from neither is a `ReferenceError` waiting for the line to run.
const AMBIENT: &[&str] = &[
    "AbortController", "Array", "ArrayBuffer", "ArrayBufferLike", "ArrayBufferView",
    "AsyncGenerator", "AsyncIterableIterator", "AsyncIterator", "Awaited", "BigInt",
    "BigInt64Array", "BigUint64Array", "Blob", "Boolean", "Buffer", "CloseEvent", "DOMException",
    "DataView", "Date", "Error", "Event", "EventTarget", "Exclude", "Extract", "Float32Array",
    "Float64Array", "Function", "Generator", "IDBCursor", "IDBDatabase", "IDBFactory", "IDBIndex",
    "IDBKeyRange", "IDBObjectStore", "IDBOpenDBRequest", "IDBRequest", "IDBTransaction",
    "Infinity", "Int16Array", "Int32Array", "Int8Array", "Intl", "IterableIterator", "Iterator",
    "JSON", "Map", "Math", "MessageEvent", "NaN", "NonNullable", "Number", "Object", "Omit",
    "Parameters", "Partial", "Pick", "Promise", "Proxy", "RangeError", "Record", "Readonly",
    "Reflect", "RegExp", "ReturnType", "Set", "String", "Symbol", "SyntaxError", "TextDecoder",
    "TextEncoder", "TypeError", "URL", "Uint16Array", "Uint32Array", "Uint8Array", "WeakMap",
    "WeakRef", "WeakSet", "WebSocket", "clearInterval", "clearTimeout", "console", "crypto",
    "fetch", "globalThis", "indexedDB", "isFinite", "isNaN", "parseFloat", "parseInt",
    "performance", "process", "queueMicrotask", "setInterval", "setTimeout", "structuredClone",
    "undefined", "FinalizationRegistry",
];

#[test]
fn no_emitted_file_names_something_it_never_declares() {
    require_eslint();
    let crates = crates_in_scope();
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (package, src) in &crates {
        let out = TempDir::new(&format!("lint-gate-{package}"));
        run_batch(src, out.path(), package);
        write_config(out.path());
        found.insert(package.clone(), lint(out.path()));
    }
    compare("undeclared names", "undefined", &found);
}

#[test]
fn no_emitted_line_calls_one_function_twice() {
    let crates = crates_in_scope();
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (package, src) in &crates {
        let out = TempDir::new(&format!("lint-gate-calls-{package}"));
        run_batch(src, out.path(), package);
        let mut rows = Vec::new();
        for (name, text) in collect_files_with_ext(out.path(), Some("ts")) {
            for (at, line) in text.lines().enumerate() {
                for call in repeated_calls(line) {
                    rows.push(format!("{name}:{}: {call}", at + 1));
                }
            }
        }
        rows.sort();
        found.insert(package.clone(), rows);
    }
    compare("repeated calls", "repeated", &found);
}

/// One eslint run over a directory of emitted files, as `rule name × count`
/// rows.
///
/// The run's working directory is the emitted directory itself: eslint's flat
/// config lints what is under its base path, and a directory somewhere else is
/// silently "all ignored".
fn lint(dir: &Path) -> Vec<String> {
    let out = Command::new(eslint_bin())
        .current_dir(dir)
        .args(["--no-config-lookup", "-c", "eslint.config.mjs", "--format", "json", "."])
        .output()
        .unwrap_or_else(|e| panic!("cannot run eslint: {e}"));
    let text = String::from_utf8_lossy(&out.stdout);
    let reports: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or_else(|e| {
        panic!(
            "eslint did not answer with JSON ({e}):\n{}\n{}",
            text,
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for report in reports.as_array().into_iter().flatten() {
        for message in report["messages"].as_array().into_iter().flatten() {
            let rule = message["ruleId"].as_str().unwrap_or("(fatal)").to_string();
            let text = message["message"].as_str().unwrap_or_default();
            *counts.entry((rule, named_by(text))).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|((rule, name), count)| format!("{rule} {name} × {count}"))
        .collect()
}

/// The identifier an eslint message is about, where the message names one.
///
/// `'Ulid' is not defined.` is about `Ulid`; a rule that names nothing keeps
/// its whole message, which is what makes the row readable on its own.
fn named_by(message: &str) -> String {
    match message.split_once('\'').and_then(|(_, rest)| rest.split_once('\'')) {
        Some((name, _)) if !name.is_empty() => name.to_string(),
        _ => message.to_string(),
    }
}

/// Every `identifier(..)` this line calls more than once.
///
/// An approximation, and deliberately a conservative one: a call written twice
/// on one line is what a lowering that reads its subject twice produces, and
/// two calls to one function on one line for any other reason are rare enough
/// to sit in the ledger. Only calls whose callee is a NAME or a member chain
/// count — `f(x)` and `a.b.c(x)` — because those are the shapes a receiver
/// takes; an anonymous arrow called twice is not one value read twice.
fn repeated_calls(line: &str) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let bytes: Vec<char> = line.chars().collect();
    for (at, ch) in bytes.iter().enumerate() {
        if *ch != '(' {
            continue;
        }
        let Some(callee) = callee_before(&bytes, at) else { continue };
        // A keyword that takes a parenthesis is not a call.
        if matches!(
            callee.as_str(),
            "if" | "for" | "while" | "switch" | "catch" | "return" | "typeof" | "function" | "new"
        ) {
            continue;
        }
        *seen.entry(callee).or_default() += 1;
    }
    seen.into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(callee, n)| format!("{callee}() × {n}"))
        .collect()
}

/// The name immediately before an open parenthesis, where one is written there.
fn callee_before(chars: &[char], at: usize) -> Option<String> {
    let mut from = at;
    while from > 0 {
        let ch = chars[from - 1];
        if ch.is_alphanumeric() || ch == '_' || ch == '$' || ch == '.' {
            from -= 1;
        } else {
            break;
        }
    }
    if from == at {
        return None;
    }
    let callee: String = chars[from..at].iter().collect();
    // A number followed by `(` is not a call, and a bare `.` is not a name.
    let first = callee.chars().next()?;
    (first.is_alphabetic() || first == '_' || first == '$').then_some(callee)
}

/// The eslint flat config the emitted files are linted under, written beside
/// them so the run is self-contained.
fn write_config(dir: &Path) {
    let globals: Vec<String> = AMBIENT.iter().map(|name| format!("  '{name}': 'readonly',")).collect();
    let parser = repo_root().join("node_modules/@typescript-eslint/parser/dist/index.js");
    let config = format!(
        "import tsParser from '{parser}';\n\
         export default [\n  {{\n    files: ['**/*.ts'],\n    \
         languageOptions: {{\n      parser: tsParser,\n      \
         parserOptions: {{ ecmaVersion: 'latest', sourceType: 'module' }},\n      \
         globals: {{\n{globals}\n      }},\n    }},\n    \
         rules: {{\n      \
         'no-undef': 'error',\n      \
         'no-dupe-keys': 'error',\n      \
         'no-unreachable': 'error',\n      \
         'no-self-assign': 'error',\n      \
         'no-constant-condition': ['error', {{ checkLoops: false }}],\n    \
         }},\n  }},\n];\n",
        parser = parser.display(),
        globals = globals.join("\n")
    );
    std::fs::write(dir.join("eslint.config.mjs"), config).expect("cannot write the eslint config");
}

fn repo_root() -> std::path::PathBuf {
    transpile_dir().parent().expect("transpile/ has a parent").to_path_buf()
}

fn eslint_bin() -> std::path::PathBuf {
    repo_root().join("node_modules/.bin/eslint")
}

fn require_eslint() {
    let bin = eslint_bin();
    assert!(
        bin.exists(),
        "this test asks ESLint which names emitted code never declares, and {} is not there. \
         Run `bun install` in the checkout.",
        bin.display()
    );
}

/// One ledger comparison, exact in both directions.
fn compare(what: &str, key: &str, found: &BTreeMap<String, Vec<String>>) {
    let path = transpile_dir().join("tests/lint_gate.toml");
    if std::env::var_os("UPDATE_LINT_GATE").is_some() {
        record(&path, key, found);
        eprintln!("updated {} [{key}]", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nRecord it with:\n    \
             cd transpile && UPDATE_LINT_GATE=1 cargo test --test lint_gate",
            path.display()
        )
    });
    let recorded: toml::Table = text.parse().expect("the lint-gate ledger is not valid TOML");
    let mut moved = String::new();
    for (package, rows) in found {
        let listed: Vec<String> = recorded
            .get(package.as_str())
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        for row in rows {
            if !listed.contains(row) {
                let _ = writeln!(moved, "\n{package}: {what}, and the ledger does not have it:\n    {row}");
            }
        }
        for row in &listed {
            if !rows.contains(row) {
                let _ = writeln!(moved, "\n{package}: the ledger records {what} that are gone. Take the line out:\n    {row}");
            }
        }
    }
    let total: usize = found.values().map(Vec::len).sum();
    assert!(
        moved.is_empty(),
        "the {what} ledger has moved ({total} rows across {} crates):\n{moved}\n\
         Fix the emitter, or — once every line above has been read and accepted — refresh with:\n    \
         cd transpile && UPDATE_LINT_GATE=1 cargo test --test lint_gate",
        found.len()
    );
}

/// Write one half of the ledger back, keeping the other half as it stands.
fn record(path: &Path, key: &str, found: &BTreeMap<String, Vec<String>>) {
    let existing: toml::Table = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_default();
    let mut out = String::from(HEADER);
    let packages: Vec<&String> = found.keys().collect();
    for package in packages {
        let _ = writeln!(out, "[{package}]");
        for half in ["undefined", "repeated"] {
            let rows: Vec<String> = if half == key {
                found.get(package).cloned().unwrap_or_default()
            } else {
                existing
                    .get(package.as_str())
                    .and_then(|v| v.as_table())
                    .and_then(|t| t.get(half))
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                    .unwrap_or_default()
            };
            if rows.is_empty() {
                let _ = writeln!(out, "{half} = []");
                continue;
            }
            let _ = writeln!(out, "{half} = [");
            for row in rows {
                let _ = writeln!(out, "  {},", toml::Value::String(row));
            }
            let _ = writeln!(out, "]");
        }
        out.push('\n');
    }
    std::fs::write(path, out).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

const HEADER: &str = "\
# Names emitted code uses without declaring them, and emitted lines that call
# one function twice — written by transpile/tests/lint_gate.rs.
#
# `undefined` rows are ESLint's, one per rule and identifier with the number of
# occurrences: `no-undef Ulid × 2`. `repeated` rows are the double-receiver
# scan's, one per emitted line. Both are matched EXACTLY in both directions: a
# new occurrence fails, and one that is gone fails until its line is taken out.
# The point is the ratchet — every row here is a `ReferenceError` waiting for
# its line to run, or a value read twice that Rust reads once.
#
# Generated: do not hand-edit. Refresh with:
#     cd transpile && UPDATE_LINT_GATE=1 cargo test --test lint_gate

";
