//! Whether the emitted port can be IMPORTED — every specifier every emitted
//! module writes, resolved against the packages the port actually has.
//!
//! For: the parse gate asks whether one file parses, and it asks with
//! `bun build --external '*'`, which makes every specifier somebody else's
//! problem. So nothing in the harness asked the next question — does the name
//! this file imports exist where it says it comes from. It did not:
//! `storage-indexeddb/collection.ts` imported `Iter`, `SortedStream` and
//! `TopKStream` from `@ankurah/core`, which exports none of the three, because
//! the import list was decided by scanning rendered text and the `collect`
//! refusal NAMES those three types in its message. The module failed to load,
//! and the harness was green.
//!
//! So this gate lays the emitted crates out as packages — one directory per
//! crate, each `@ankurah/<crate>` in a node_modules beside them, with
//! `@ankurah/base` pointing at the runtime and each package's hand-written
//! half underneath the emitted output, which is the layout the validation copy
//! uses — and runs `bun build` over every emitted module with NO `--external`.
//! A specifier that resolves to nothing and a named import the target does not
//! export both fail the build, and the error names the file and line where the
//! import is written, so an entry point failing on something three modules
//! away is recorded where the defect is rather than where the walk started.
//!
//! Two ledgers, each matched exactly in both directions like the diagnostics
//! budget: an import that starts naming nothing fails, and one that stops
//! fails until its line comes out.
//!
//!     cd transpile && UPDATE_IMPORT_GATE=1 cargo test --test import_gate

mod common;

use common::{collect_files_with_ext, crates_in_scope, run_batch, transpile_dir, TempDir};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Every import an emitted module writes names a module that exists and a
/// symbol that module offers.
#[test]
fn every_emitted_import_resolves_against_the_emitted_packages() {
    require_bun();
    let root = TempDir::new("import-gate");
    let modules = lay_out_packages(root.path());
    assert!(
        modules.len() > 100,
        "the import gate laid out {} emitted modules across ten crates, which is too few to be \
         the port: the layout step is wrong, not the emission",
        modules.len()
    );

    let mut unresolved: BTreeSet<String> = BTreeSet::new();
    let mut unexported: BTreeSet<String> = BTreeSet::new();
    for module in &modules {
        for row in build(root.path(), module) {
            match row {
                Complaint::Unresolved(text) => unresolved.insert(text),
                Complaint::Unexported(text) => unexported.insert(text),
            };
        }
    }

    let ledger = transpile_dir().join("tests/import_gate.toml");
    if std::env::var_os("UPDATE_IMPORT_GATE").is_some() {
        std::fs::write(&ledger, render(&unresolved, &unexported))
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", ledger.display()));
        eprintln!("updated {}", ledger.display());
        return;
    }
    let text = std::fs::read_to_string(&ledger).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nRecord it with:\n    \
             cd transpile && UPDATE_IMPORT_GATE=1 cargo test --test import_gate",
            ledger.display()
        )
    });
    let recorded: toml::Table = text.parse().expect("the import ledger is not valid TOML");

    let mut moved = String::new();
    compare("a specifier that names nothing", &unresolved, listed(&recorded, "unresolved"), &mut moved);
    compare("a name its module does not export", &unexported, listed(&recorded, "unexported"), &mut moved);
    assert!(
        moved.is_empty(),
        "which emitted imports resolve has moved ({} unresolved specifier(s), {} unexported \
         name(s), over {} modules):\n{moved}\nFix the emitter, or — once every line above has \
         been read and accepted — refresh with:\n    \
         cd transpile && UPDATE_IMPORT_GATE=1 cargo test --test import_gate",
        unresolved.len(),
        unexported.len(),
        modules.len()
    );
}

/// The one module K1 was found in loads: `storage-indexeddb/collection.ts`
/// imported three iterator types from a package that exports none of them.
///
/// The ledgers above are a list; this is the assertion that the file the
/// review found is not on it.
#[test]
fn the_indexeddb_collection_imports_only_names_that_exist() {
    require_bun();
    let root = TempDir::new("import-gate-collection");
    lay_out_packages(root.path());
    let module = PathBuf::from("pkg/storage-indexeddb/src/collection.ts");
    let complaints: Vec<String> = build(root.path(), &module)
        .into_iter()
        .filter_map(|c| match c {
            Complaint::Unexported(text) => Some(text),
            // A `[[provided]]` module the port has not written yet is the
            // ledger's business, not this test's.
            Complaint::Unresolved(_) => None,
        })
        .collect();
    assert!(
        complaints.is_empty(),
        "storage-indexeddb/collection.ts imports {} name(s) nothing exports:\n  {}",
        complaints.len(),
        complaints.join("\n  ")
    );
}

enum Complaint {
    Unresolved(String),
    Unexported(String),
}

/// Emit every crate in scope into `<root>/pkg/<crate>/src`, over the
/// hand-written half each package already carries, and link every package
/// under `<root>/node_modules/@ankurah` so a `@ankurah/core` specifier
/// resolves to the emitted core. Answers the modules to check, each relative
/// to `root`.
fn lay_out_packages(root: &Path) -> Vec<PathBuf> {
    let repo = transpile_dir().parent().expect("the transpiler sits in the repo").to_path_buf();
    let links = root.join("node_modules/@ankurah");
    std::fs::create_dir_all(&links).expect("the node_modules layout is writable");
    let mut modules = Vec::new();
    let mut packages: Vec<String> = Vec::new();
    for (package, src) in crates_in_scope() {
        let dir = root.join("pkg").join(&package);
        let out = dir.join("src");
        std::fs::create_dir_all(&out).expect("the package layout is writable");
        // The hand-written half — `id.provided.ts`, the ankql parser, the
        // bincode codec — travels with main's packages, and the emitted output
        // goes over it, as the validation copy is built. ONLY the hand-written
        // half: a file the batch no longer writes must not be left behind by
        // an older emission to answer an import that should have failed.
        let from = repo.join("packages").join(&package).join("src");
        for name in hand_written_half(&from, &src) {
            let target = out.join(&name);
            if let Some(dir) = target.parent() {
                std::fs::create_dir_all(dir).expect("the package layout is writable");
            }
            let _ = std::fs::copy(from.join(&name), target);
        }
        run_batch(&src, &out, &package);
        std::fs::write(
            dir.join("package.json"),
            format!("{{\n  \"name\": \"@ankurah/{package}\",\n  \"main\": \"src/index.ts\"\n}}\n"),
        )
        .expect("the package manifest is writable");
        link(&dir, &links.join(&package));
        for name in collect_files_with_ext(&out, Some("ts")).keys() {
            modules.push(PathBuf::from("pkg").join(&package).join("src").join(name));
        }
        packages.push(package);
    }
    assert!(packages.len() >= 10, "the port's scope is ten crates, and the layout has {}", packages.len());
    // The runtime is not emitted: it is `packages/base`, written by hand.
    link(&repo.join("packages/base"), &links.join("base"));
    modules
}

/// What of a package's `src` is written by hand, as paths under it: every
/// `*.provided.ts` (which is how `[provided_impls]` spells its `path`), the
/// bincode codec, and every module `transpile.toml` declares `[[provided]]` or
/// `[[extra_exports]]` for THIS crate — `util/cb_future`, `connection`,
/// `parser`. The batch writes none of these, and without them an import that
/// names one resolves to nothing and the ledger says the port owes a file it
/// has already written.
fn hand_written_half(package_src: &Path, crate_src: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    collect_provided_files(package_src, package_src, &mut out);
    let text = std::fs::read_to_string(transpile_dir().join("transpile.toml"))
        .expect("transpile.toml is readable");
    let config: toml::Table = text.parse().expect("transpile.toml is valid TOML");
    for key in ["provided", "extra_exports"] {
        let Some(entries) = config.get(key).and_then(|v| v.as_array()) else { continue };
        for entry in entries {
            let (Some(file), Some(module)) = (
                entry.get("file").and_then(|v| v.as_str()),
                entry.get("module").and_then(|v| v.as_str()),
            ) else {
                continue;
            };
            // `storage/indexeddb-wasm/src/util/cb_future.rs` belongs to the
            // crate whose source directory ends with `storage/indexeddb-wasm/src`.
            let Some((dir, _)) = file.rsplit_once("/src/") else { continue };
            if !crate_src.to_string_lossy().ends_with(&format!("{dir}/src")) {
                continue;
            }
            // `util/index` is the directory's own module, written `util/index.ts`.
            out.push(PathBuf::from(format!("{module}.ts")));
        }
    }
    out.retain(|name| package_src.join(name).is_file());
    out.sort();
    out.dedup();
    out
}

fn collect_provided_files(root: &Path, dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_provided_files(root, &path, into);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".provided.ts") || name == "codec.ts" {
            if let Ok(rel) = path.strip_prefix(root) {
                into.push(rel.to_path_buf());
            }
        }
    }
}

fn link(target: &Path, at: &Path) {
    if at.exists() {
        return;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, at)
        .unwrap_or_else(|e| panic!("cannot link {} to {}: {e}", at.display(), target.display()));
    #[cfg(not(unix))]
    panic!(
        "the import gate needs a symlink to put {} in a node_modules, and this platform has none",
        target.display()
    );
}


/// What `bun build` says about one module, as the file and line each complaint
/// names rather than the entry point the walk started at.
fn build(root: &Path, module: &Path) -> Vec<Complaint> {
    let output = Command::new("bun")
        .arg("build")
        .arg(module)
        .arg("--target=bun")
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("cannot run bun build on {}: {e}", module.display()));
    if output.status.success() && output.stderr.is_empty() {
        return Vec::new();
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let mut out = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (at, line) in lines.iter().enumerate() {
        let line = line.trim();
        let Some(complaint) = line.strip_prefix("error: ") else { continue };
        // The line below an error names the file and column it happened in:
        //     at /abs/path/pkg/core/src/index.ts:27:15
        let site = lines
            .get(at + 1)
            .map(|l| l.trim())
            .and_then(|l| l.strip_prefix("at "))
            .map(|l| where_in_the_port(root, l))
            .unwrap_or_else(|| "(bun named no file)".to_string());
        let row = format!("{site}: {complaint}");
        if complaint.starts_with("Could not resolve") {
            out.push(Complaint::Unresolved(row));
        } else {
            out.push(Complaint::Unexported(row));
        }
    }
    out
}

/// `/tmp/import-gate-xyz/pkg/core/src/index.ts:27:15` as `core/index.ts:27`,
/// so the ledger reads like the rest of the harness and does not carry a
/// temporary directory's name. The cut is at the layout's own `/pkg/` rather
/// than at the root's path: macOS hands `bun` the `/private`-prefixed spelling
/// of the same temporary directory, so a prefix match answers nothing.
fn where_in_the_port(_root: &Path, site: &str) -> String {
    let trimmed = match site.rfind("/pkg/") {
        Some(at) => &site[at + "/pkg/".len()..],
        None => site,
    };
    let trimmed = trimmed.replace("/src/", "/");
    // Drop the column; a line is as precise as a ledger wants to be.
    match trimmed.rsplit_once(':') {
        Some((head, _column)) => head.to_string(),
        None => trimmed,
    }
}

fn listed(recorded: &toml::Table, key: &str) -> BTreeSet<String> {
    recorded
        .get(key)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn compare(what: &str, found: &BTreeSet<String>, listed: BTreeSet<String>, into: &mut String) {
    for row in found.difference(&listed) {
        let _ = writeln!(into, "  new — {what}: {row}");
    }
    for row in listed.difference(found) {
        let _ = writeln!(into, "  gone, take the line out: {row}");
    }
}

fn render(unresolved: &BTreeSet<String>, unexported: &BTreeSet<String>) -> String {
    let mut out = String::from(
        "# Imports the emitted port writes that do not resolve, written by\n\
         # transpile/tests/import_gate.rs. `unresolved` is a specifier naming no module —\n\
         # every one of these is a `[[provided]]` module the port has not written yet.\n\
         # `unexported` is a name its module does not offer, which is a defect in the\n\
         # import list. Matched EXACTLY in both directions. Generated: do not hand-edit.\n\
         # Refresh with:\n\
         #     cd transpile && UPDATE_IMPORT_GATE=1 cargo test --test import_gate\n\n",
    );
    for (key, rows) in [("unresolved", unresolved), ("unexported", unexported)] {
        let _ = writeln!(out, "# {}: {}", key, rows.len());
        if rows.is_empty() {
            let _ = writeln!(out, "{key} = []\n");
            continue;
        }
        let _ = writeln!(out, "{key} = [");
        for row in rows {
            let _ = writeln!(out, "  {row:?},");
        }
        out.push_str("]\n\n");
    }
    out
}

fn require_bun() {
    match Command::new("bun").arg("--version").output() {
        Ok(probe) if probe.status.success() => {}
        Ok(probe) => panic!("`bun --version` failed ({}):\n{}", probe.status, String::from_utf8_lossy(&probe.stderr)),
        Err(e) => panic!(
            "cannot run `bun`: {e}. This gate asks a JavaScript engine whether the emitted \
             imports resolve, so bun has to be on PATH."
        ),
    }
}
