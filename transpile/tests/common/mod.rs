//! Helpers shared by the transpiler's integration tests.
//!
//! Three jobs: find the read-only Rust corpus, drive the `batch` subcommand the
//! same way a person does from a shell, and print a difference between two texts
//! so a failure tells you what changed instead of dumping both files.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// What a hand-written class DECLARES, for the checks that keep a
/// `[provided_impls]` claim honest.
pub mod members;

/// Whether an import the port writes can be followed: the two tools that
/// answer, and the manifest question neither of them asks.
pub mod imports;

/// The import gate's recorded lists: read, compared and written back.
pub mod gate_ledger;

/// The transpiler's own package directory. `transpile.toml` is read relative to
/// the current directory, so every `batch` run below starts here — the same
/// place a person stands when they run `cargo run -- batch ...` by hand.
pub fn transpile_dir() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")) }

/// The ankurah Rust checkout the corpus tests read. `ANKURAH_SUPPORT_PATH`
/// overrides; otherwise we look for `ankurah-ts-support` beside one of our own
/// ancestors, which finds it from the main checkout and from a git worktree
/// alike.
pub fn support_tree() -> PathBuf {
    if let Some(p) = std::env::var_os("ANKURAH_SUPPORT_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_dir(), "ANKURAH_SUPPORT_PATH is not a directory: {}", p.display());
        return p;
    }
    for ancestor in transpile_dir().ancestors() {
        let candidate = ancestor.join("ankurah-ts-support");
        if candidate.join("proto/src").is_dir() {
            return candidate;
        }
    }
    panic!(
        "cannot find the ankurah-ts-support checkout above {}; \
         set ANKURAH_SUPPORT_PATH to point at it",
        transpile_dir().display()
    );
}

/// Path to the transpiler binary cargo just built for this test run.
pub fn transpile_bin() -> &'static str { env!("CARGO_BIN_EXE_ankurah-transpile") }

/// Run `batch` over a source directory, writing TypeScript into `out_dir`.
/// Panics with the transpiler's own stderr when it fails, because a batch that
/// died halfway would otherwise show up as a confusing snapshot difference.
pub fn run_batch(src_dir: &Path, out_dir: &Path, crate_name: &str) {
    let output = Command::new(transpile_bin())
        .current_dir(transpile_dir())
        .arg("batch")
        .arg(src_dir)
        .arg(out_dir)
        .arg("--crate-name")
        .arg(crate_name)
        .output()
        .unwrap_or_else(|e| panic!("failed to run the transpiler binary: {e}"));
    assert!(
        output.status.success(),
        "batch {} failed ({}):\n{}",
        crate_name,
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Run `batch` and hand back everything it wrote to stderr: the diagnostics
/// list and the summary line. Same failure behaviour as `run_batch`.
pub fn run_batch_capturing(src_dir: &Path, out_dir: &Path, crate_name: &str) -> String {
    let output = Command::new(transpile_bin())
        .current_dir(transpile_dir())
        .arg("batch")
        .arg(src_dir)
        .arg(out_dir)
        .arg("--crate-name")
        .arg(crate_name)
        .output()
        .unwrap_or_else(|e| panic!("failed to run the transpiler binary: {e}"));
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "batch {} failed ({}):\n{}", crate_name, output.status, stderr);
    stderr
}

/// A scratch directory that deletes itself when the test drops it.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ankurah-transpile-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap_or_else(|e| panic!("cannot create {}: {e}", path.display()));
        TempDir(path)
    }

    pub fn path(&self) -> &Path { &self.0 }
}

impl Drop for TempDir {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

/// Every file under `root`, keyed by its slash-separated path relative to
/// `root`, sorted so two trees compare in a stable order.
pub fn collect_files(root: &Path) -> BTreeMap<String, String> { collect_files_with_ext(root, None) }

/// The same, restricted to one file extension.
pub fn collect_files_with_ext(root: &Path, ext: Option<&str>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    collect_into(root, root, ext, &mut out);
    out
}

fn collect_into(root: &Path, dir: &Path, ext: Option<&str>, out: &mut BTreeMap<String, String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_into(root, &path, ext, out);
        } else {
            if let Some(want) = ext {
                if path.extension().map_or(true, |e| e != want) {
                    continue;
                }
            }
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            out.insert(rel, text);
        }
    }
}

/// Trailing whitespace and a missing final newline are not differences worth
/// failing a test over, so both sides are squared up before comparison.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// A unified diff of two texts, `label` naming the file. Empty when they match.
pub fn unified_diff(label: &str, expected: &str, actual: &str) -> String {
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    if a == b {
        return String::new();
    }
    let mut out = format!("--- expected {label}\n+++ actual   {label}\n");
    for (i, line) in edit_script(&a, &b).into_iter().enumerate() {
        // Cap runaway diffs: a wholesale rewrite is already obvious by line 400.
        if i >= 400 {
            out.push_str("... (diff truncated at 400 lines)\n");
            break;
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Longest-common-subsequence edit script with three lines of context around
/// each change. Corpus files are a few hundred lines, so the quadratic table is
/// cheaper than pulling in a diff library.
fn edit_script(a: &[&str], b: &[&str]) -> Vec<String> {
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![0u32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[at(i, j)] = if a[i] == b[j] {
                lcs[at(i + 1, j + 1)] + 1
            } else {
                lcs[at(i + 1, j)].max(lcs[at(i, j + 1)])
            };
        }
    }

    // Walk the table into a tagged line list, then keep only changed lines and
    // their neighbours.
    let mut tagged: Vec<(char, &str)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            tagged.push((' ', a[i]));
            i += 1;
            j += 1;
        } else if lcs[at(i + 1, j)] >= lcs[at(i, j + 1)] {
            tagged.push(('-', a[i]));
            i += 1;
        } else {
            tagged.push(('+', b[j]));
            j += 1;
        }
    }
    while i < n {
        tagged.push(('-', a[i]));
        i += 1;
    }
    while j < m {
        tagged.push(('+', b[j]));
        j += 1;
    }

    const CONTEXT: usize = 3;
    let keep: Vec<bool> = (0..tagged.len())
        .map(|k| {
            let lo = k.saturating_sub(CONTEXT);
            let hi = (k + CONTEXT).min(tagged.len() - 1);
            (lo..=hi).any(|x| tagged[x].0 != ' ')
        })
        .collect();

    let mut out = Vec::new();
    let mut skipping = false;
    for (k, (tag, line)) in tagged.iter().enumerate() {
        if keep[k] {
            skipping = false;
            out.push(format!("{tag}{line}"));
        } else if !skipping {
            skipping = true;
            out.push("@@".to_string());
        }
    }
    out
}

/// One row of `resolve`: which function a method call landed on.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Path relative to the support checkout, e.g. `signals/src/broadcast.rs`.
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub method: String,
    pub receiver: String,
    pub adjusted: String,
    pub callee: String,
    pub result: String,
    /// `from|to` for each dereference taken, in order, separated by `;`.
    pub steps: Vec<(String, String)>,
}

/// Ask the engine which function every method call in a crate resolves to.
pub fn run_resolve(crate_name: &str, src_rel: &str) -> Vec<Resolved> {
    let output = Command::new(transpile_bin())
        .current_dir(transpile_dir())
        .arg("resolve")
        .arg(support_tree().join(src_rel))
        .arg("--crate-name")
        .arg(crate_name)
        .output()
        .unwrap_or_else(|e| panic!("failed to run the transpiler binary: {e}"));
    assert!(
        output.status.success(),
        "resolve {crate_name} failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let prefix = format!("{}/", src_rel);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("RESOLVED\t"))
        .map(|row| {
            let f: Vec<&str> = row.split('\t').collect();
            assert!(f.len() >= 9, "malformed resolve row: {row}");
            Resolved {
                file: format!("{prefix}{}", f[0]),
                line: f[1].parse().unwrap_or(0),
                col: f[2].parse().unwrap_or(0),
                method: f[3].to_string(),
                receiver: f[4].to_string(),
                adjusted: f[5].to_string(),
                callee: f[6].to_string(),
                result: f[7].to_string(),
                steps: f[8]
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| s.split_once('|'))
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
            }
        })
        .collect()
}

/// One row of `resolve`'s closure record: the signature the engine gave a
/// closure written at that position.
#[derive(Debug, Clone)]
pub struct ClosureRow {
    pub file: String,
    pub line: u32,
    pub col: u32,
    /// One entry per parameter; `None` where the engine could not type it.
    pub params: Vec<Option<String>>,
    /// The return type, or `None` where the engine could not say.
    pub ret: Option<String>,
}

/// Ask the engine what signature it gave every closure in a crate.
pub fn run_closures(crate_name: &str, src_rel: &str) -> Vec<ClosureRow> {
    let output = Command::new(transpile_bin())
        .current_dir(transpile_dir())
        .arg("resolve")
        .arg(support_tree().join(src_rel))
        .arg("--crate-name")
        .arg(crate_name)
        .output()
        .unwrap_or_else(|e| panic!("failed to run the transpiler binary: {e}"));
    assert!(
        output.status.success(),
        "resolve {crate_name} failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let prefix = format!("{}/", src_rel);
    let unknown = |s: &str| (s != "?").then(|| s.to_string());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("CLOSURE\t"))
        .map(|row| {
            let f: Vec<&str> = row.split('\t').collect();
            assert!(f.len() >= 5, "malformed closure row: {row}");
            ClosureRow {
                file: format!("{prefix}{}", f[0]),
                line: f[1].parse().unwrap_or(0),
                col: f[2].parse().unwrap_or(0),
                params: if f[3].is_empty() {
                    Vec::new()
                } else {
                    f[3].split(';').map(unknown).collect()
                },
                ret: unknown(f[4]),
            }
        })
        .collect()
}

/// One `?` the engine saw convert: the two error types, and the function it
/// wrote to convert them where it could name one.
pub struct TryRow {
    pub file: String,
    pub line: u32,
    pub from: String,
    pub to: String,
    pub written: Option<String>,
}

/// Ask the engine what every `?` in a crate converts, and how.
pub fn run_tries(crate_name: &str, src_rel: &str) -> Vec<TryRow> {
    let output = Command::new(transpile_bin())
        .current_dir(transpile_dir())
        .arg("resolve")
        .arg(support_tree().join(src_rel))
        .arg("--crate-name")
        .arg(crate_name)
        .output()
        .unwrap_or_else(|e| panic!("failed to run the transpiler binary: {e}"));
    assert!(
        output.status.success(),
        "resolve {crate_name} failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let prefix = format!("{}/", src_rel);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("TRYCONV\t"))
        .map(|row| {
            let f: Vec<&str> = row.split('\t').collect();
            assert!(f.len() >= 6, "malformed try row: {row}");
            TryRow {
                file: format!("{prefix}{}", f[0]),
                line: f[1].parse().unwrap_or(0),
                from: f[3].to_string(),
                to: f[4].to_string(),
                written: (f[5] != "-").then(|| f[5].to_string()),
            }
        })
        .collect()
}

/// Every crate in the port's scope, with the directory `batch` is pointed at.
///
/// The list comes from `transpile.toml` rather than from a table here, so a
/// crate entering or leaving the port's scope moves the tests that walk it.
/// Where each crate's sources sit comes from the crate's own `Cargo.toml`, the
/// way the engine's sibling loader finds them.
pub fn crates_in_scope() -> Vec<(String, PathBuf)> {
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
pub fn manifests_under(root: &Path) -> BTreeMap<String, PathBuf> {
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

/// One thing being read while a TypeScript file is scanned.
///
/// A template literal's `${..}` is CODE, so the states nest: a string inside an
/// interpolation inside a template. `Code` carries the brace depth opened since
/// the frame began, so the `}` that closes an interpolation can be told from
/// one that closes a block inside it.
enum Reading {
    Code(usize),
    LineComment,
    BlockComment,
    Quoted(char),
}

/// The file with every comment and every string literal blanked out: each of
/// their characters replaced by a space, every newline kept.
///
/// For: a harness check that reads TypeScript with `find` and `contains` is
/// reading text, and text inside a string literal is not code. A
/// `const a = "static fromJson(";` satisfied `declared_members`'s
/// `reads_json` check on its own; a `{` or a `}` inside a string moved the
/// brace depth that decides where a class ends, so a later class's members were
/// read as this one's; and the comment scan that ran before did not know about
/// strings either, so a `//` inside one swallowed the rest of a real line.
///
/// Blanked rather than removed, so every offset and every line number the
/// caller computes still lands where it did. The braces of a `${..}` are kept
/// for the same reason: they balance, and a scan counting depth must see them.
/// A regular-expression literal is read as division, which is what every other
/// simple TypeScript scanner does; nothing in the corpus or the provided files
/// writes one.
pub fn code_only(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut stack: Vec<Reading> = vec![Reading::Code(0)];
    let mut at = 0usize;
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    while at < chars.len() {
        let ch = chars[at];
        let next = chars.get(at + 1).copied();
        match stack.last_mut().expect("the outermost frame is never popped") {
            Reading::Code(depth) => match (ch, next) {
                ('/', Some('/')) => {
                    stack.push(Reading::LineComment);
                    out.push(' ');
                    at += 1;
                }
                ('/', Some('*')) => {
                    stack.push(Reading::BlockComment);
                    out.push(' ');
                    at += 1;
                }
                ('"', _) | ('\'', _) | ('`', _) => {
                    stack.push(Reading::Quoted(ch));
                    out.push(ch);
                    at += 1;
                }
                ('{', _) => {
                    *depth += 1;
                    out.push(ch);
                    at += 1;
                }
                ('}', _) => {
                    // A `}` with nothing open in this frame closes the `${..}`
                    // that opened it, and hands the template back its quote.
                    if *depth > 0 {
                        *depth -= 1;
                    } else if stack.len() > 1 {
                        stack.pop();
                    }
                    out.push(ch);
                    at += 1;
                }
                _ => {
                    out.push(ch);
                    at += 1;
                }
            },
            Reading::LineComment => {
                out.push(blank(ch));
                at += 1;
                if ch == '\n' {
                    stack.pop();
                }
            }
            Reading::BlockComment => {
                out.push(blank(ch));
                at += 1;
                if ch == '*' && next == Some('/') {
                    out.push(' ');
                    at += 1;
                    stack.pop();
                }
            }
            Reading::Quoted(quote) => {
                let quote = *quote;
                if ch == '\\' {
                    out.push(' ');
                    at += 1;
                    if at < chars.len() {
                        out.push(blank(chars[at]));
                        at += 1;
                    }
                } else if ch == quote {
                    out.push(ch);
                    at += 1;
                    stack.pop();
                } else if quote == '`' && ch == '$' && next == Some('{') {
                    out.push(' ');
                    out.push('{');
                    at += 2;
                    stack.push(Reading::Code(0));
                } else {
                    out.push(blank(ch));
                    at += 1;
                }
            }
        }
    }
    out
}
