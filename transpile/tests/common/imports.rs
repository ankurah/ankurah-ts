//! The three questions the import gate asks about one laid-out port, and the
//! one row shape all three answer in.
//!
//! For: `bun build` is blind to an import whose names are all used in TYPE
//! position. It erases the import before resolution, so it never opens the
//! module and never checks the export — two lines prove it: `import { Nothing }
//! from './does-not-exist'; export function f(x: Nothing): void {}` builds
//! clean. The gate's ledger therefore recorded ten unresolved specifiers where
//! the port writes twenty, and two unexported names where it writes about
//! thirty. A type-only import that names nothing is a module the port owes just
//! as much as a value one, and at runtime under a type-stripping loader it is
//! the same missing file.
//!
//! So `tsc --noEmit` runs over the same laid-out root and its import diagnostics
//! join bun's in one ledger. `TS2307` is a specifier naming no module; `TS2305`,
//! `TS2459`, `TS2724` and `TS2614` are a name the module does not offer (absent,
//! declared but not exported, misspelled, or exported as a default). Both tools
//! feed the SAME two lists, in one canonical row shape — file, line, and the
//! specifier or name at issue — so a defect both tools see is one row and not
//! two.
//!
//! A third question is the manifest's: the layout links every package into one
//! shared `node_modules`, so a cross-package import passes whether or not the
//! importing package DECLARES that dependency. Expo installs by manifest, where
//! an undeclared dependency does not resolve at all. So every bare `@ankurah/*`
//! specifier an emitted package writes is checked against that package's
//! `dependencies`/`peerDependencies` in the repository's own `package.json`.
//!
//! Out of scope, and deliberately: a specifier that is neither relative nor
//! `@ankurah/*`. `bun:test`, `yjs` and the node builtins are the environment's
//! business, not the port's, and the gate would otherwise record a row for
//! every emitted test file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Which list a complaint belongs on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The specifier names no module.
    Unresolved,
    /// The module is there and the name is not.
    Unexported,
    /// Something else a tool said about an import. Its own list, so a kind
    /// nobody has read yet cannot hide inside one of the other two.
    Other,
}

/// One complaint, as the ledger writes it: `core/context.ts:7: "./indexel"
/// names no module`. The site and subject are what both tools agree on, so the
/// same defect seen by bun and by tsc is one row.
pub fn row(site: &str, kind: Kind, subject: &str) -> (Kind, String) {
    let text = match kind {
        Kind::Unresolved => format!("{site}: {subject:?} names no module"),
        Kind::Unexported => format!("{site}: {subject:?} is not exported"),
        Kind::Other => format!("{site}: {subject}"),
    };
    (kind, text)
}

/// Is this specifier the port's own business? Relative, or one of the port's
/// packages. Everything else belongs to the environment the port runs in.
pub fn is_the_ports_own(specifier: &str) -> bool {
    specifier.starts_with('.') || specifier.starts_with("@ankurah/")
}

/// What one `bun build` complaint means, as a row — or nothing, when it is
/// about a specifier the port does not own.
///
/// `message` is the text after `error: `, `site` the already-shortened
/// `core/context.ts:7`.
pub fn from_bun(site: &str, message: &str) -> Option<(Kind, String)> {
    if let Some(rest) = message.strip_prefix("Could not resolve: ") {
        let specifier = unquote(rest.trim());
        return is_the_ports_own(&specifier).then(|| row(site, Kind::Unresolved, &specifier));
    }
    if message.starts_with("No matching export") {
        // `No matching export in "pkg/core/src/lineage.ts" for import "Comparison"`
        let name = message.rsplit('"').nth(1).unwrap_or_default().to_string();
        return Some(row(site, Kind::Unexported, &name));
    }
    Some(row(site, Kind::Other, message))
}

/// Every import diagnostic `tsc` has about the laid-out port, as rows.
///
/// One run over the whole layout rather than one per module: tsc reads the
/// graph once, and a per-module run would type-check core eighty times.
pub fn tsc_rows(root: &Path, repo: &Path) -> Vec<(Kind, String)> {
    write_tsconfig(root, repo);
    let tsc = repo.join("node_modules/typescript/bin/tsc");
    assert!(
        tsc.is_file(),
        "the import gate asks `tsc` which imports resolve, and {} is not there: run `bun install` \
         in the checkout",
        tsc.display()
    );
    let output = Command::new("node")
        .arg(&tsc)
        .arg("--noEmit")
        .arg("--pretty")
        .arg("false")
        .arg("-p")
        .arg(root.join("tsconfig.json"))
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("cannot run tsc over the laid-out port: {e}"));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Reading a row back to its cause means reading what the tool actually
    // said, and the layout is a temporary directory that deletes itself.
    if let Some(path) = std::env::var_os("IMPORT_GATE_TSC_LOG") {
        std::fs::write(path, &text).expect("the tsc log path is writable");
    }
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((site, complaint)) = split_tsc_line(line) else { continue };
        let Some((kind, subject)) = tsc_subject(&complaint) else { continue };
        if !is_the_ports_own(&subject) && kind == Kind::Unresolved {
            continue;
        }
        out.push(row(&site, kind, &subject));
    }
    out
}

/// `pkg/core/src/context.ts(7,22): error TS2307: Cannot find module …` as
/// `("core/context.ts:7", "TS2307: Cannot find module …")`.
///
/// The cut is at the layout's own `pkg/` wherever it falls, because tsc spells
/// the same file two ways in one run: relative to the config for a file the
/// `include` glob found, and through the real path for the same file reached a
/// second time along a `node_modules` symlink — on macOS a walk back out of
/// `/var/folders/…` seven directories deep. Both spellings become the one site,
/// so the defect is one row.
pub fn split_tsc_line(line: &str) -> Option<(String, String)> {
    let (head, rest) = line.split_once("): error ")?;
    let (path, position) = head.rsplit_once('(')?;
    let line_number = position.split(',').next()?;
    let at = path.rfind("pkg/")?;
    let path = path[at + "pkg/".len()..].replace("/src/", "/");
    Some((format!("{path}:{line_number}"), rest.to_string()))
}

/// Which of tsc's import diagnostics this is, and what it is about.
pub fn tsc_subject(complaint: &str) -> Option<(Kind, String)> {
    let (code, message) = complaint.split_once(": ")?;
    // The gate is an import gate: a diagnostic about a BODY — a wrong argument
    // type, a missing property — is not its business, and filing those would
    // make the `other` list a second type-error budget.
    if !is_an_import_code(code) {
        return None;
    }
    match code {
        // Cannot find module './indexel' or its corresponding type declarations.
        "TS2307" => Some((Kind::Unresolved, single_quoted(message, 0)?)),
        // Module '"./ivec"' has no exported member 'Iter'.
        // '"./x"' has no exported member named 'Y'. Did you mean 'Z'?
        // Module '"./x"' has no exported member 'Y'. Did you mean to use
        // 'import Y from "./x"' instead?
        "TS2305" | "TS2724" | "TS2614" => Some((Kind::Unexported, single_quoted(message, 1)?)),
        // Module '"./lineage"' declares 'Comparison' locally, but it is not exported.
        "TS2459" => Some((Kind::Unexported, single_quoted(message, 1)?)),
        // S5: the two shapes that used to escape, and then everything else.
        //
        // Module '"./x"' has no default export.
        "TS1192" => Some((Kind::Unexported, "default".to_string())),
        // Namespace '"./x"' has no exported member 'Y'.
        "TS2694" => Some((Kind::Unexported, single_quoted(message, 1)?)),
        // Every OTHER import code is filed as `Other` rather than dropped.
        // Dropping it meant a type-only namespace use could pass the gate
        // unrecorded: the gate reported what it recognised, which is not the
        // same as reporting what tsc said.
        _ => Some((Kind::Other, format!("{code}: {message}"))),
    }
}

/// Is this tsc code about an IMPORT at all?
///
/// The gate is an import gate: a diagnostic about a body — a wrong argument
/// type, a missing property — is not its business, and filing those as `Other`
/// would make the `other` list a second type-error budget. The codes here are
/// the ones tsc emits for a module specifier or for a name read out of one.
pub fn is_an_import_code(code: &str) -> bool {
    matches!(
        code,
        "TS1192" // no default export
            | "TS1259" // esModuleInterop needed for a default import
            | "TS1261" // casing differs from the file on disk
            | "TS2305" // no exported member
            | "TS2307" // cannot find module
            | "TS2308" // two modules export the same name
            | "TS2440" // an import conflicts with a local declaration
            | "TS2459" // declared locally but not exported
            | "TS2614" // no exported member, meant a default import
            | "TS2691" // an import path must not end in .ts
            | "TS2694" // a namespace has no such member
            | "TS2724" // no exported member, did you mean
    )
}

/// The `n`th `'…'` of a tsc message, with the `"` tsc wraps a specifier in
/// taken off: tsc writes a module as `'"./lineage"'`.
fn single_quoted(message: &str, n: usize) -> Option<String> {
    let mut parts = message.split('\'');
    parts.next()?;
    let mut found = None;
    for (at, part) in parts.step_by(2).enumerate() {
        if at == n {
            found = Some(part.to_string());
            break;
        }
    }
    Some(unquote(&found?))
}

fn unquote(text: &str) -> String {
    text.trim().trim_matches('"').trim_matches('\'').trim_matches('"').to_string()
}

/// The layout's tsconfig. `types: []` keeps the ambient `@types` packages of
/// the checkout out of it — the gate asks about the port's imports, not about
/// whether the port type-checks — and `strict: false` keeps a type error from
/// costing a run its diagnostics.
fn write_tsconfig(root: &Path, repo: &Path) {
    let base = repo.join("packages/base/src/index.ts");
    let config = format!(
        r#"{{
  "compilerOptions": {{
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "noEmit": true,
    "strict": false,
    "skipLibCheck": true,
    "allowImportingTsExtensions": true,
    "lib": ["ES2022", "DOM"],
    "types": [],
    "baseUrl": ".",
    "paths": {{ "@ankurah/base": [{base:?}] }}
  }},
  "include": ["pkg/**/*.ts"]
}}
"#
    );
    std::fs::write(root.join("tsconfig.json"), config).expect("the layout root is writable");
}

/// Every `@ankurah/<other>` an emitted package imports that its manifest does
/// not declare, as ledger rows.
///
/// The layout links every package into ONE `node_modules`, and so does the
/// workspace the tests run in, so an undeclared dependency resolves in both and
/// fails only where the packages are installed one at a time — which is the
/// Expo target.
pub fn undeclared_dependencies(
    root: &Path,
    modules: &[PathBuf],
    repo: &Path,
) -> BTreeSet<String> {
    let mut wanted: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for module in modules {
        let Ok(text) = std::fs::read_to_string(root.join(module)) else { continue };
        let Some(package) = module.iter().nth(1).map(|p| p.to_string_lossy().to_string()) else {
            continue;
        };
        for specifier in bare_ankurah_specifiers(&text) {
            let site = module.to_string_lossy().replace("pkg/", "").replace("/src/", "/");
            wanted.entry(package.clone()).or_default().entry(specifier).or_insert(site);
        }
    }
    let mut out = BTreeSet::new();
    for (package, specifiers) in wanted {
        let manifest = repo.join("packages").join(&package).join("package.json");
        let declared = declared_dependencies(&manifest);
        for (specifier, site) in specifiers {
            if specifier == format!("@ankurah/{package}") || declared.contains(&specifier) {
                continue;
            }
            out.insert(format!(
                "{package}: {site} imports {specifier:?}, which packages/{package}/package.json \
                 declares in neither dependencies nor peerDependencies"
            ));
        }
    }
    out
}

/// The `@ankurah/<name>` specifiers a module imports.
///
/// A specifier inside a string the emission wrote for a MESSAGE is not an
/// import, so the scan is anchored on the keyword that begins a declaration —
/// but a declaration is not a line. S6: reading one line at a time, requiring
/// that line to begin with `import`/`export`, and splitting on single quotes
/// only, missed `import { X } from "@ankurah/x";` and every multi-line named
/// import, whose `} from '@ankurah/x';` begins with a brace. The manifest check
/// then said a package declared everything it needed while an undeclared
/// dependency stood two lines down.
///
/// So: find each `import`/`export` KEYWORD at the start of a statement, then
/// read forward to the first quote of either kind and take what is inside it.
pub fn bare_ankurah_specifiers(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for start in declaration_starts(text) {
        let rest = &text[start..];
        // A declaration ends at its semicolon or at the newline that follows
        // the specifier; either way the specifier is the first quoted run.
        let end = rest.find(';').unwrap_or(rest.len());
        if let Some(specifier) = first_quoted(&rest[..end]) {
            if specifier.starts_with("@ankurah/") {
                out.insert(specifier);
            }
        }
    }
    out
}

/// Where each `import`/`export` statement begins: the keyword at the start of a
/// line, or after a `;` or `}` — never inside an identifier and never inside a
/// string the emission wrote.
fn declaration_starts(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    for keyword in ["import", "export"] {
        let mut from = 0;
        while let Some(at) = text[from..].find(keyword) {
            let at = from + at;
            from = at + keyword.len();
            let before = text[..at].trim_end();
            let begins = before.is_empty()
                || before.ends_with(';')
                || before.ends_with('}')
                || text[..at].ends_with('\n')
                || before.len() < text[..at].trim_end_matches([' ', '\t']).len();
            let after_is_part = bytes
                .get(from)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
            if begins && !after_is_part {
                out.push(at);
            }
        }
    }
    out
}

/// The first single- or double-quoted run in this text.
fn first_quoted(text: &str) -> Option<String> {
    let at = text.find(['\'', '"'])?;
    let quote = text.as_bytes()[at] as char;
    let rest = &text[at + 1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn declared_dependencies(manifest: &Path) -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(manifest) else { return BTreeSet::new() };
    let mut out = BTreeSet::new();
    // A hand-rolled read of two objects, so the harness does not gain a JSON
    // dependency for eleven manifests: every `"@ankurah/x": "…"` line inside
    // `dependencies` or `peerDependencies`.
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"dependencies\"") || trimmed.starts_with("\"peerDependencies\"") {
            inside = true;
            continue;
        }
        if inside && trimmed.starts_with('}') {
            inside = false;
            continue;
        }
        if inside {
            if let Some(name) = trimmed.split('"').nth(1) {
                out.insert(name.to_string());
            }
        }
    }
    out
}

