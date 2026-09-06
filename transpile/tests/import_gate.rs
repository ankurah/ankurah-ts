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
//! What the tool delivers, exactly. `bun build` answers for an import whose
//! names are USED as values, and for no other: an import all of whose names are
//! used in type position is erased before resolution, so bun never opens the
//! module and never checks the export (`import { Nothing } from
//! './does-not-exist'; export function f(x: Nothing): void {}` builds clean).
//! Ten of the port's twenty dead specifiers and most of its unexported names
//! were type-only, and the ledger recorded none of them. So `tsc --noEmit` runs
//! over the SAME laid-out root and its import diagnostics join bun's in the same
//! two lists, in one canonical row shape, deduplicated: see
//! `tests/common/imports.rs` for which diagnostic codes count and why a
//! non-`@ankurah`, non-relative specifier is out of scope.
//!
//! And a third question neither tool asks: the layout links every package into
//! ONE `node_modules`, so a cross-package import resolves whether or not the
//! importing package DECLARES that dependency — while Expo installs by
//! manifest, where it does not resolve at all. `undeclared_dependencies` is that
//! list.
//!
//! Four ledgers, each matched exactly in both directions like the diagnostics
//! budget: an import that starts naming nothing fails, and one that stops
//! fails until its line comes out. The lists are a RATCHET — they may shrink
//! and never grow — and the counts in `[summary]` are what a review reads.
//!
//!     cd transpile && UPDATE_IMPORT_GATE=1 cargo test --test import_gate

mod common;

use common::gate_ledger::{compare, listed, render};
use common::imports::{self, Kind};
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
    let mut other: BTreeSet<String> = BTreeSet::new();
    let mut file = |kind: Kind, text: String| {
        match kind {
            Kind::Unresolved => unresolved.insert(text),
            Kind::Unexported => unexported.insert(text),
            Kind::Other => other.insert(text),
        };
    };
    for module in &modules {
        for (kind, text) in build(root.path(), module) {
            file(kind, text);
        }
    }
    // The same lists, asked of the tool that can see a type-only import.
    for (kind, text) in imports::tsc_rows(root.path(), &repo()) {
        file(kind, text);
    }
    let undeclared = imports::undeclared_dependencies(root.path(), &modules, &repo());

    let ledger = transpile_dir().join("tests/import_gate.toml");
    if std::env::var_os("UPDATE_IMPORT_GATE").is_some() {
        std::fs::write(&ledger, render(&unresolved, &unexported, &other, &undeclared))
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

    // A `[summary]` that disagrees with the lists below it is a ledger lying
    // about its own size, which is the one thing a ratchet is read for.
    for (key, found) in [
        ("unresolved", unresolved.len()),
        ("unexported", unexported.len()),
        ("other", other.len()),
        ("undeclared_dependencies", undeclared.len()),
    ] {
        let recorded_rows = listed(&recorded, key).len();
        let summarised = recorded
            .get("summary")
            .and_then(|v| v.as_table())
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_integer())
            .unwrap_or(-1);
        assert_eq!(
            summarised, recorded_rows as i64,
            "the ledger's [summary] says {key} = {summarised} and lists {recorded_rows} row(s) \
             (the gate found {found})"
        );
    }

    let mut moved = String::new();
    compare("a specifier that names nothing", &unresolved, listed(&recorded, "unresolved"), &mut moved);
    compare("a name its module does not export", &unexported, listed(&recorded, "unexported"), &mut moved);
    compare("something else a tool said about an import", &other, listed(&recorded, "other"), &mut moved);
    compare(
        "a cross-package import the manifest does not declare",
        &undeclared,
        listed(&recorded, "undeclared_dependencies"),
        &mut moved,
    );
    assert!(
        moved.is_empty(),
        "which emitted imports resolve has moved ({} unresolved specifier(s), {} unexported \
         name(s), {} other complaint(s), {} undeclared dependency/ies, over {} modules):\n\
         {moved}\nFix the emitter, or — once every line above has been read and accepted — \
         refresh with:\n    \
         cd transpile && UPDATE_IMPORT_GATE=1 cargo test --test import_gate",
        unresolved.len(),
        unexported.len(),
        other.len(),
        undeclared.len(),
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
        // A `[[provided]]` module the port has not written yet is the ledger's
        // business, not this test's.
        .filter(|(kind, _)| *kind == Kind::Unexported)
        .map(|(_, text)| text)
        .collect();
    assert!(
        complaints.is_empty(),
        "storage-indexeddb/collection.ts imports {} name(s) nothing exports:\n  {}",
        complaints.len(),
        complaints.join("\n  ")
    );
}


/// Emit every crate in scope into `<root>/pkg/<crate>/src`, over the
/// hand-written half each package already carries, and link every package
/// under `<root>/node_modules/@ankurah` so a `@ankurah/core` specifier
/// resolves to the emitted core. Answers the modules to check, each relative
/// to `root`.
fn lay_out_packages(root: &Path) -> Vec<PathBuf> {
    let repo = repo();
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
            // `types` as well as `main`: tsc reads `types` to find a package's
            // entry point, and without it `@ankurah/core` resolves to nothing
            // for the tool that can see the type-only imports.
            format!(
                "{{\n  \"name\": \"@ankurah/{package}\",\n  \"main\": \"src/index.ts\",\n  \
                 \"types\": \"src/index.ts\"\n}}\n"
            ),
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

/// The checkout the emitted port's hand-written halves, its runtime and its
/// package manifests come from.
fn repo() -> PathBuf {
    transpile_dir().parent().expect("the transpiler sits in the repo").to_path_buf()
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
fn build(root: &Path, module: &Path) -> Vec<(Kind, String)> {
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
        if let Some(row) = imports::from_bun(&site, complaint) {
            out.push(row);
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

// How a tool's complaint becomes a ledger row. These live here rather than
// beside the code they exercise because `tests/common` is compiled into every
// test binary, and a unit test there would run fourteen times.
#[test]
fn a_bun_complaint_becomes_a_row() {
    assert_eq!(
        imports::from_bun("core/context.ts:7", "Could not resolve: \"./indexel\""),
        Some((Kind::Unresolved, "core/context.ts:7: \"./indexel\" names no module".into()))
    );
    assert_eq!(
        imports::from_bun(
            "core/lineage.test.ts:4",
            "No matching export in \"pkg/core/src/lineage.ts\" for import \"Comparison\""
        ),
        Some((Kind::Unexported, "core/lineage.test.ts:4: \"Comparison\" is not exported".into()))
    );
    // A third kind of complaint gets its own list rather than being filed
    // under `unexported`, which is where every non-resolution error used to
    // go.
    assert_eq!(
        imports::from_bun("core/x.ts:1", "Unexpected end of file").map(|(k, _)| k),
        Some(Kind::Other)
    );
    // A specifier the port does not own is the environment's business.
    assert_eq!(imports::from_bun("core/x.test.ts:1", "Could not resolve: \"bun:test\""), None);
}

#[test]
fn a_tsc_diagnostic_becomes_the_same_row() {
    let bun = imports::from_bun("core/context.ts:7", "Could not resolve: \"./indexel\"").unwrap();
    let line = "pkg/core/src/context.ts(7,22): error TS2307: Cannot find module './indexel' \
                or its corresponding type declarations.";
    let (site, complaint) = imports::split_tsc_line(line).unwrap();
    assert_eq!(site, "core/context.ts:7");
    // The same file reached a second time along a node_modules symlink, which
    // tsc spells as a walk out of the real temporary directory: one site, so
    // one row.
    let through_the_link = "../../../../../var/folders/dm/T/import-gate-1/pkg/core/src/\
                            context.ts(7,22): error TS2307: Cannot find module './indexel' or \
                            its corresponding type declarations.";
    assert_eq!(imports::split_tsc_line(through_the_link).unwrap().0, site);
    let (kind, subject) = imports::tsc_subject(&complaint).unwrap();
    assert_eq!(imports::row(&site, kind, &subject), bun, "one defect, one row, whichever tool saw it");
}

#[test]
fn the_four_unexported_shapes_name_the_missing_name() {
    let cases = [
        (
            "pkg/core/src/util/iterable.ts(3,10): error TS2305: Module '\"./ivec\"' has no \
             exported member 'Iter'.",
            "core/util/iterable.ts:3: \"Iter\" is not exported",
        ),
        (
            "pkg/core/src/node.ts(10,10): error TS2459: Module '\"./lineage\"' declares \
             'Comparison' locally, but it is not exported.",
            "core/node.ts:10: \"Comparison\" is not exported",
        ),
        (
            "pkg/storage-indexeddb/src/collection.ts(4,10): error TS2724: '\"@ankurah/core\"' \
             has no exported member named 'Iter'. Did you mean 'IVec'?",
            "storage-indexeddb/collection.ts:4: \"Iter\" is not exported",
        ),
        (
            "pkg/ankql/src/parser.ts(2,10): error TS2614: Module '\"./ast\"' has no exported \
             member 'Expr'. Did you mean to use 'import Expr from \"./ast\"' instead?",
            "ankql/parser.ts:2: \"Expr\" is not exported",
        ),
    ];
    for (line, expected) in cases {
        let (site, complaint) = imports::split_tsc_line(line).unwrap_or_else(|| panic!("{line}"));
        let (kind, subject) = imports::tsc_subject(&complaint).unwrap_or_else(|| panic!("{line}"));
        assert_eq!(kind, Kind::Unexported);
        assert_eq!(imports::row(&site, kind, &subject).1, expected);
    }
}

#[test]
fn a_type_error_is_not_an_import_diagnostic() {
    let line = "pkg/core/src/node.ts(88,7): error TS2339: Property 'x' does not exist on \
                type 'Y'.";
    let (_, complaint) = imports::split_tsc_line(line).unwrap();
    assert_eq!(imports::tsc_subject(&complaint), None);
}

#[test]
fn an_import_line_names_its_cross_package_specifiers() {
    let text = "import { Enum } from '@ankurah/base';\n\
                export { x } from '@ankurah/proto';\n\
                const message = `see '@ankurah/nothing'`;\n";
    let found = imports::bare_ankurah_specifiers(text);
    assert!(found.contains("@ankurah/base") && found.contains("@ankurah/proto"));
    assert!(!found.contains("@ankurah/nothing"), "a name inside a message is not an import");
}

/// S5: the two import shapes that used to escape the classifier, and the rule
/// that nothing else does.
///
/// `imports::tsc_subject` returned `None` for every code it did not recognise, so a
/// missing default export and a namespace import's missing member passed the
/// gate unrecorded — a type-only namespace use COULD escape.
#[test]
fn a_missing_default_and_a_namespace_member_are_both_filed() {
    let default = "pkg/core/src/node.ts(3,8): error TS1192: Module '\"./thing\"' has no \
                   default export.";
    let (site, complaint) = imports::split_tsc_line(default).unwrap();
    assert_eq!(site, "core/node.ts:3");
    assert_eq!(
        imports::tsc_subject(&complaint),
        Some((imports::Kind::Unexported, "default".to_string())),
        "the missing name IS `default`"
    );

    let member = "pkg/core/src/node.ts(9,14): error TS2694: Namespace '\"./thing\"' has no \
                  exported member 'Held'.";
    let (_, complaint) = imports::split_tsc_line(member).unwrap();
    assert_eq!(
        imports::tsc_subject(&complaint),
        Some((imports::Kind::Unexported, "Held".to_string())),
        "and here it is the member the namespace does not offer"
    );
}

/// Any OTHER import code is filed as `Other` rather than dropped: the gate
/// reports what tsc said, not only what it recognises.
#[test]
fn an_import_code_nobody_has_read_yet_is_filed_as_other() {
    let line = "pkg/core/src/node.ts(4,1): error TS2308: Module './a' has already exported a \
                member named 'X'.";
    let (_, complaint) = imports::split_tsc_line(line).unwrap();
    let (kind, subject) = imports::tsc_subject(&complaint).unwrap();
    assert_eq!(kind, imports::Kind::Other);
    assert!(subject.starts_with("TS2308: "), "the code travels with it: {subject}");
}

/// S6: a specifier is read off a DECLARATION, not off a line.
///
/// Reading one line at a time, requiring that line to begin with
/// `import`/`export`, and splitting on single quotes only, missed a
/// double-quoted specifier and every multi-line named import — whose
/// `} from '@ankurah/x';` begins with a brace.
#[test]
fn a_specifier_is_read_across_newlines_and_both_quotes() {
    let text = "import { X } from \"@ankurah/double\";\n\
                import {\n  A,\n  B,\n} from '@ankurah/multiline';\n\
                import '@ankurah/side-effect';\n\
                import type { T } from '@ankurah/types-only';\n\
                export * from \"@ankurah/re-exported\";\n\
                export { y } from '@ankurah/named-re-export';\n";
    let found = imports::bare_ankurah_specifiers(text);
    for expected in [
        "@ankurah/double",
        "@ankurah/multiline",
        "@ankurah/side-effect",
        "@ankurah/types-only",
        "@ankurah/re-exported",
        "@ankurah/named-re-export",
    ] {
        assert!(found.contains(expected), "{expected} was not read: {found:?}");
    }
}

/// And a name inside a MESSAGE the emission wrote is still not an import,
/// however it is quoted.
#[test]
fn a_specifier_inside_a_message_is_still_not_an_import() {
    let text = "import { Enum } from '@ankurah/base';\n\
                const message = `see \"@ankurah/nothing\"`;\n\
                const other = 'exported from \"@ankurah/neither\"';\n";
    let found = imports::bare_ankurah_specifiers(text);
    assert!(found.contains("@ankurah/base"));
    assert!(!found.contains("@ankurah/nothing"), "{found:?}");
    assert!(!found.contains("@ankurah/neither"), "{found:?}");
}
