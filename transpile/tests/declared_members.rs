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

use common::members::{class_members, declares_instance_method, declares_nullary_method, declares_static_method, Member};
use common::{code_only, collect_files_with_ext, run_batch, support_tree, transpile_dir, TempDir};
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

        // Read as CODE, not as text: a `.fromJson(` inside a string literal is
        // not a call and a `static fromJson(` inside one is not a declaration.
        let files: BTreeMap<String, String> =
            files.into_iter().map(|(name, text)| (name, code_only(&text))).collect();

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

/// Z13: this check reads TypeScript, and text inside a string literal is not
/// code. Every one of these used to pass the scan.
#[test]
fn a_member_named_inside_a_string_does_not_declare_it() {
    let file = "export class Wrong {\n  readonly a = \"static fromJson( { }\";\n}\n\
                export class Right {\n  static fromJson(v: unknown) { return v; }\n  \
                toJSON() { return 1; }\n}\n";
    let wrong = class_members(file, "Wrong").expect("Wrong is declared");
    assert!(!declares_static_method(&wrong, "fromJson"), "a string satisfied the check: {wrong:?}");
    assert!(!declares_instance_method(&wrong, "toJSON"), "a brace in a string ended the class late: {wrong:?}");
    let right = class_members(file, "Right").expect("Right is declared");
    assert!(declares_static_method(&right, "fromJson"), "{right:?}");
    assert!(declares_instance_method(&right, "toJSON"), "{right:?}");
}

/// E2: a CALL is not a declaration.
///
/// The gate read the class body as one string and asked whether `debug()`
/// appeared in it, so a class with no `debug()` of its own but one method that
/// calls somebody else's satisfied the claim — which is how the reviewer deleted
/// `EntityId.debug()`, left a caller behind, and watched the gate stay green.
#[test]
fn a_call_does_not_satisfy_the_claim() {
    let file = "export class Caller {\n  render(): string { return this.inner.debug(); }\n  \
                stash(): unknown { return other.toJSON(); }\n}\n";
    let members = class_members(file, "Caller").expect("Caller is declared");
    assert!(!declares_nullary_method(&members, "debug"), "a call satisfied has_debug: {members:?}");
    assert!(!declares_instance_method(&members, "toJSON"), "a call satisfied reads_json: {members:?}");
    assert_eq!(
        members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        vec!["render", "stash"],
        "the members are the two the class writes: {members:?}"
    );
}

/// F8: a STATIC is not the member the emission calls.
///
/// `#[derive(Debug)]` on a type holding a provided one writes `value.debug()`,
/// and a `static debug()` leaves that undefined. `fromJson` is the mirror: the
/// emitter writes `Class.fromJson(v)`, so an INSTANCE `fromJson` does not
/// answer it either.
#[test]
fn a_static_does_not_satisfy_an_instance_claim_or_the_reverse() {
    let file = "export class Wrong {\n  static debug(): string { return \"x\"; }\n  \
                fromJson(v: unknown) { return v; }\n}\n";
    let members = class_members(file, "Wrong").expect("Wrong is declared");
    assert!(!declares_nullary_method(&members, "debug"), "a static satisfied has_debug: {members:?}");
    assert!(!declares_static_method(&members, "fromJson"), "an instance method satisfied reads_json: {members:?}");
    assert_eq!(members[0], Member { name: "debug".into(), is_static: true, params: Some(String::new()) });
    assert_eq!(members[1], Member { name: "fromJson".into(), is_static: false, params: Some("v: unknown".into()) });
}

/// A declaration NESTED inside a member is not this class's.
///
/// A local class, an object literal with a method shorthand, and a class
/// expression assigned to a field all write the same words one level down.
#[test]
fn a_nested_declaration_is_not_the_class_s_own() {
    let file = "export class Outer {\n  build(): unknown {\n    class Inner { debug(): string { return \"i\"; } }\n    \
                const shim = { debug() { return \"s\"; }, toJSON() { return 0; } };\n    \
                return [Inner, shim];\n  }\n  \
                readonly proxy = class { static fromJson(v: unknown) { return v; } };\n}\n";
    let members = class_members(file, "Outer").expect("Outer is declared");
    assert!(!declares_nullary_method(&members, "debug"), "a nested declaration satisfied has_debug: {members:?}");
    assert!(!declares_instance_method(&members, "toJSON"), "an object literal satisfied reads_json: {members:?}");
    assert!(!declares_static_method(&members, "fromJson"), "a class expression satisfied reads_json: {members:?}");
    assert_eq!(
        members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        vec!["build", "proxy"],
        "{members:?}"
    );
}

/// The shapes a real provided file writes, read back with the kind and the
/// parameter list each was written with.
#[test]
fn the_declaration_shapes_a_provided_file_writes() {
    let file = "export class Shapes extends Struct {\n  \
                readonly _0: string;\n  \
                private cache?: Map<string, number>;\n  \
                constructor(v: string) { super(); this._0 = v; }\n  \
                debug(): string { return `Shapes(${this._0})`; }\n  \
                toJSON(): string { return this._0; }\n  \
                static fromJson<T>(value: unknown): Result<Shapes, JsonError> { return Result.Ok(new Shapes(\"\")); }\n  \
                get length(): number { return this._0.length; }\n  \
                compareTo(other: Shapes): number { return 0; }\n}\n";
    let members = class_members(file, "Shapes").expect("Shapes is declared");
    assert!(declares_nullary_method(&members, "debug"), "{members:?}");
    assert!(declares_instance_method(&members, "toJSON"), "{members:?}");
    assert!(declares_static_method(&members, "fromJson"), "{members:?}");
    // A field declares no parameter list, so it is never mistaken for a method.
    let fields: Vec<&str> =
        members.iter().filter(|m| m.params.is_none()).map(|m| m.name.as_str()).collect();
    assert_eq!(fields, vec!["_0", "cache"], "{members:?}");
    // `get length()` is a member named `length`, not a member named `get`.
    assert!(members.iter().any(|m| m.name == "length"), "{members:?}");
    assert!(!members.iter().any(|m| m.name == "get"), "{members:?}");
    // The generic static's parameter list is the one after its type parameters.
    let from_json = members.iter().find(|m| m.name == "fromJson").expect("{members:?}");
    assert_eq!(from_json.params.as_deref(), Some("value: unknown"));
}

/// `get(key)` is a member called `get`; the word is a modifier only where a name
/// follows it.
#[test]
fn a_modifier_word_used_as_a_name_is_a_name() {
    let file = "export class Keyed {\n  get(key: string): number { return 0; }\n  \
                set(key: string, v: number): void {}\n  static get instance(): Keyed { return new Keyed(); }\n}\n";
    let members = class_members(file, "Keyed").expect("Keyed is declared");
    assert!(declares_instance_method(&members, "get"), "{members:?}");
    assert!(declares_instance_method(&members, "set"), "{members:?}");
    assert!(members.iter().any(|m| m.name == "instance" && m.is_static), "{members:?}");
}

/// A one-line summary of what a class declares, for the failure message: a
/// claim that fails should say what the file has instead.
fn summarise(members: &[Member]) -> String {
    if members.is_empty() {
        return "nothing".to_string();
    }
    members
        .iter()
        .map(|m| {
            let kind = if m.is_static { "static " } else { "" };
            match &m.params {
                Some(p) => format!("{kind}{}({p})", m.name),
                None => format!("{kind}{} (a field)", m.name),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A `//` inside a string used to swallow the rest of a real line, and a
/// `.fromJson(` inside one used to count as a call. Offsets and line counts are
/// preserved, because callers slice by them.
#[test]
fn a_comment_marker_inside_a_string_is_not_a_comment() {
    let code = code_only("const url = \"http://x\"; A.fromJson(v);\n");
    assert!(code.contains("A.fromJson("), "the rest of the line was eaten:\n{code}");
    assert!(from_json_receivers(&code_only("const s = \"B.fromJson(\";\n")).is_empty());
    let text = "a\n\"b\"\n// c\n";
    assert_eq!(code_only(text).len(), text.len());
    assert_eq!(code_only(text).lines().count(), text.lines().count());
}

/// A template's `${..}` is CODE: a call written inside one is a call, and its
/// braces balance so a scan counting depth still sees them.
#[test]
fn a_template_interpolation_is_code() {
    let code = code_only("const s = `x ${A.fromJson(v)} y`;\n");
    assert_eq!(from_json_receivers(&code), vec!["A".to_string()]);
    assert!(code.contains("{") && code.contains("}"), "{code}");
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
        // The CLASS, not the file. `id.provided.ts` declares six classes and six
        // statics, so a claim about any one of them passed as long as some other
        // class in the same file had one — which is the "unverified claim one
        // indirection further on" this check exists to stop.
        let members = class_members(&text, &class).unwrap_or_else(|| {
            panic!(
                "[provided_impls] {fqn} says `reads_json = true`, but {} declares no \
                 `export class {class}`.",
                file.display()
            )
        });
        // BOTH halves, each as a DECLARATION of the kind the emission calls.
        // §4.2's contract is that the pair is refused as one, so a file with
        // `fromJson` and no `toJSON` would let `x.toJSON()` be emitted against
        // nothing; and a `toJSON(` a method body merely CALLS says nothing about
        // what this class declares.
        assert!(
            declares_static_method(&members, "fromJson"),
            "[provided_impls] {fqn} says `reads_json = true`, but class `{class}` in {} declares \
             no `static fromJson(..)` of its own. Emitted readers write `{class}.fromJson(v)`.\n\
             What it does declare: {}",
            file.display(),
            summarise(&members)
        );
        assert!(
            declares_instance_method(&members, "toJSON"),
            "[provided_impls] {fqn} says `reads_json = true`, but class `{class}` in {} declares \
             no `toJSON(..)` instance method. Emitted writers write `value.toJSON()`.\n\
             What it does declare: {}",
            file.display(),
            summarise(&members)
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

/// Every `debug()` the emitter writes on a HAND-WRITTEN type is one the file
/// really declares.
///
/// The same rule `reads_json` follows, for the other member a `[provided_impls]`
/// entry can claim. `#[derive(Debug)]` on a type holding a provided one printed
/// the field through `toString` — which for a class is `[object Object]` — and
/// the entry saying `has_debug = true` is what turns that into a real call, so
/// an entry beside a file with no such method has to fail here.
#[test]
fn every_has_debug_claim_names_a_declared_method() {
    let table = config_table();
    let provided = table
        .get("provided_impls")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("transpile.toml has no [provided_impls] table"));
    let crates = table
        .get("crates")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("transpile.toml has no [crates] table"));

    let mut checked = 0usize;
    for (fqn, entry) in provided {
        let entry = entry.as_table().unwrap_or_else(|| panic!("[provided_impls] {fqn} is not a table"));
        if !entry.get("has_debug").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        let path = entry
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("[provided_impls] {fqn} has no `path`"));
        let class = fqn.rsplit("::").next().unwrap_or(fqn).to_string();
        let file = provided_file(crates, fqn, path);
        let text = std::fs::read_to_string(&file).unwrap_or_else(|e| {
            panic!("[provided_impls] {fqn} names {}, which cannot be read: {e}", file.display())
        });
        let members = class_members(&text, &class).unwrap_or_else(|| {
            panic!(
                "[provided_impls] {fqn} says `has_debug = true`, but {} declares no \
                 `export class {class}`.",
                file.display()
            )
        });
        assert!(
            declares_nullary_method(&members, "debug"),
            "[provided_impls] {fqn} says `has_debug = true`, but class `{class}` in {} declares \
             no `debug()` instance method taking nothing. Emitted `Debug` lines write \
             `value.debug()`, which a call inside another method, a `static debug()` and a \
             `debug(depth)` all leave undefined.\n\
             What it does declare: {}",
            file.display(),
            summarise(&members)
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no [provided_impls] entry says `has_debug = true`, so this check is proving nothing"
    );
}
