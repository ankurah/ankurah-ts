//! Top-level TS code generation — orchestrates imports, emission, and output

mod surface;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::registry::TypeRegistry;
use crate::types::*;
use crate::emit;
use crate::imports;

/// Generate TypeScript skeleton with resolved imports (used by batch command)
pub fn generate_ts_with_imports_configured(
    reg: &TypeRegistry,
    file: &RustFile,
    rust_crate_path: &str,
    type_to_file: &HashMap<String, String>,
    current_module: &str,
    config: Option<&crate::config::Config>,
) -> String {
    let base = generate_ts_inner(reg, file, rust_crate_path, config);

    let mut local_types: HashSet<String> = HashSet::new();
    for s in &file.structs { local_types.insert(s.name.clone()); }
    for e in &file.enums { local_types.insert(e.name.clone()); }
    for t in &file.traits { local_types.insert(t.name.clone()); }
    // A module-level function this file declares for one of its own impls is
    // not imported into itself.
    if let Some(module) = reg.modules().lookup_file(&file.path) {
        for f in crate::emit_impls::free_functions(reg, module, file) {
            local_types.insert(f.name);
        }
        for d in crate::emit_impls::dispatchers(reg, module, file) {
            local_types.insert(d.name);
        }
    }

    // Collect all referenced types — including from function/method bodies
    let mut referenced: HashSet<String> = HashSet::new();
    for s in &file.structs {
        for f in &s.fields { imports::collect_type_refs(&f.ts_ty(reg), &mut referenced); }
    }
    for e in &file.enums {
        for v in &e.variants {
            for f in &v.fields { imports::collect_type_refs(&f.ts_ty(reg), &mut referenced); }
        }
    }
    // An impl whose target this file does not declare has its methods emitted
    // onto that type's class, which is written where the type is — so nothing
    // of it reaches this file and importing what its signature names would
    // import a symbol nothing here uses. (An impl written for a type another
    // file declares is emitted nowhere at all today; that is its own gap.)
    let declares = |name: &String| {
        file.structs.iter().any(|s| s.name == *name) || file.enums.iter().any(|e| e.name == *name)
    };
    let emitted_here = |imp: &ImplInfo| match reg.modules().lookup_file(&file.path) {
        Some(module) => {
            declares(&imp.target_type) || !crate::emit_impls::impl_has_class(reg, module, imp)
        }
        None => true,
    };
    for imp in file.impls.iter().filter(|imp| emitted_here(imp)) {
        for m in &imp.methods {
            imports::collect_type_refs(&m.return_type, &mut referenced);
            imports::collect_type_refs(&m.generics, &mut referenced);
            for p in &m.params { imports::collect_type_refs(&p.ty, &mut referenced); }
            if let Some(b) = &m.body_ts { imports::collect_type_refs(b, &mut referenced); }
        }
    }
    for f in &file.functions {
        imports::collect_type_refs(&f.return_type, &mut referenced);
        imports::collect_type_refs(&f.generics, &mut referenced);
        for p in &f.params { imports::collect_type_refs(&p.ty, &mut referenced); }
        if let Some(b) = &f.body_ts { imports::collect_type_refs(b, &mut referenced); }
    }
    for decl in &file.module_decls {
        imports::collect_type_refs(decl, &mut referenced);
    }
    // Trait names from `implements` clauses
    for imp in &file.impls {
        if let Some(trait_name) = imp.trait_name() {
            referenced.insert(trait_name);
        }
    }

    // A call to an impl emitted as a module-level function names it in
    // camelCase, which the type scan above skips on purpose, so those names are
    // matched whole against the ones the run emitted.
    let free_names: HashSet<String> = type_to_file
        .keys()
        .filter(|name| name.chars().next().is_some_and(|c| c.is_lowercase()))
        .cloned()
        .collect();
    if !free_names.is_empty() {
        let bodies = file
            .impls
            .iter()
            .flat_map(|imp| imp.methods.iter())
            .chain(file.functions.iter())
            .filter_map(|f| f.body_ts.as_deref());
        for body in bodies {
            imports::collect_named_refs(body, &free_names, &mut referenced);
        }
    }

    // Group external types by source module
    let mut imports_by_module: HashMap<String, Vec<String>> = HashMap::new();
    for type_name in &referenced {
        if local_types.contains(type_name) || imports::is_primitive_or_base_type(type_name) {
            continue;
        }
        if let Some(source_module) = type_to_file.get(type_name) {
            if source_module != current_module {
                imports_by_module.entry(source_module.clone())
                    .or_default()
                    .push(type_name.clone());
            }
        }
    }

    // Build import lines, ONE per module specifier.
    //
    // Several Rust modules reach the same TypeScript module: `ankurah_core::
    // policy`, `ankurah_core::node` and `ankurah_core::connector` are all
    // `@ankurah/core`, because a crate is one package here. Written one line per
    // RUST module that gave the specifier twice — `import { Node, PeerSender,
    // PolicyAgent, .. } from '@ankurah/core'` immediately followed by
    // `import { Node, PeerSender, .. } from '@ankurah/core'` — which is eight
    // `TS2300` duplicate identifiers in connector-local alone, and every one of
    // them a name the file genuinely needs.
    let mut by_specifier: std::collections::BTreeMap<String, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for (module, types) in &imports_by_module {
        let specifier = relative_import_path(current_module, module);
        by_specifier.entry(specifier).or_default().extend(types.iter().cloned());
    }

    // Import functions from inline modules.
    // Scan bodies for function names that exist in inline modules.
    for (mod_name, sub_file) in file
        .inline_modules
        .iter()
        .filter(|(name, _)| Some(name) != file.test_module.as_ref())
    {
        let sub_module = format!("{}/{}", current_module.trim_end_matches("/index"), mod_name);
        let func_names: std::collections::HashSet<String> = sub_file.functions.iter()
            .map(|f| f.ts_name.clone()).collect();
        let mut found: Vec<String> = Vec::new();
        let all_bodies = file.impls.iter()
            .flat_map(|imp| imp.methods.iter())
            .chain(file.functions.iter())
            .filter_map(|f| f.body_ts.as_deref());
        for body in all_bodies {
            for word in body.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if func_names.contains(word) && !found.contains(&word.to_string()) {
                    found.push(word.to_string());
                }
            }
        }
        if !found.is_empty() {
            let import_path = relative_import_path(current_module, &sub_module);
            by_specifier.entry(import_path).or_default().extend(found);
        }
    }

    // Replace the TODO imports line, and merge every named import by the module
    // it names.
    //
    // Two passes write import lines: this one, from the types the file's text
    // refers to, and `generate_ts_inner`, from the `use` statements the file
    // wrote. Both reach the same package — a crate is ONE TypeScript module
    // here, so `use ankurah_core::node::Node` and `use ankurah_core::
    // connector::PeerSender` are both `@ankurah/core` — and each wrote its own
    // line. connector-local's emitted file opened with two
    // `import { .. } from '@ankurah/core'` lines sharing four names: eight
    // `TS2300` duplicate identifiers, which was every own-file error that
    // package had.
    merge_named_imports(&base, &by_specifier)
}

/// One `import { .. } from '<module>'` per module, everything else untouched.
///
/// `written` is the file as the emitters left it and `extra` the names this
/// pass resolved. Named imports are collected from both, merged per module and
/// written back where the first import stood; a namespace import, a side-effect
/// import and every other line keep their place and their order.
fn merge_named_imports(
    written: &str,
    extra: &std::collections::BTreeMap<String, BTreeSet<String>>,
) -> String {
    // Per module, the names in the order they were first written. The order is
    // the emitter's — `Struct, Enum, Result, ..`, the base primitives first —
    // and sorting it would rewrite every emitted file and every golden for
    // nothing.
    let mut names: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut body = String::new();
    let mut marker: Option<usize> = None;
    let mut add = |names: &mut std::collections::BTreeMap<String, Vec<String>>, module: &str, name: String| {
        let list = names.entry(module.to_string()).or_default();
        if !list.contains(&name) {
            list.push(name);
        }
    };
    for line in written.lines() {
        if line.starts_with("// TODO imports:") {
            marker.get_or_insert(body.len());
            continue;
        }
        match named_import(line) {
            Some((module, imported)) => {
                marker.get_or_insert(body.len());
                if !order.contains(&module) {
                    order.push(module.clone());
                }
                for name in imported {
                    add(&mut names, &module, name);
                }
            }
            None => {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    for (module, imported) in extra {
        if !order.contains(module) {
            order.push(module.clone());
        }
        for name in imported {
            add(&mut names, module, name.clone());
        }
    }

    // One name from two modules is a `TS2300` duplicate identifier, and the
    // file will not compile: `entity.ts` imports `State` from `@ankurah/proto`
    // and from `./reactor/subscription_state`, because Rust told the two apart
    // by the path each was written with and this pass wrote both bare. Said out
    // loud rather than emitted in silence.
    let mut seen: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
    for (module, imported) in &names {
        for name in imported {
            seen.entry(name.as_str()).or_default().push(module.as_str());
        }
    }
    for (name, modules) in seen {
        if modules.len() > 1 {
            crate::diag::pending::park_at(
                0,
                0,
                format!(
                    "`{}` is imported from {} at once, and TypeScript has one name here where \
                     Rust had two paths",
                    name,
                    modules.join(" and ")
                ),
            );
        }
    }

    let mut lines = String::new();
    for module in &order {
        let Some(imported) = names.get(module) else { continue };
        if imported.is_empty() {
            continue;
        }
        let joined: Vec<&str> = imported.iter().map(String::as_str).collect();
        let _ = std::fmt::Write::write_fmt(
            &mut lines,
            format_args!("import {{ {} }} from '{}';\n", joined.join(", "), module),
        );
    }
    match marker {
        Some(at) => {
            let mut out = String::with_capacity(body.len() + lines.len());
            out.push_str(&body[..at]);
            out.push_str(&lines);
            out.push_str(&body[at..]);
            out
        }
        None => format!("{lines}{body}"),
    }
}

/// The module and the names an `import { A, B } from 'm';` line brings in.
fn named_import(line: &str) -> Option<(String, Vec<String>)> {
    let rest = line.strip_prefix("import {")?;
    let (inside, after) = rest.split_once('}')?;
    let module = after.split_once('\'')?.1.split_once('\'')?.0;
    let names = inside
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect();
    Some((module.to_string(), names))
}

/// Generate TypeScript skeleton from extracted Rust file
/// `config` is optional — when provided, skips types/methods listed in provided_impls
/// Every name `@ankurah/base` exports that emitted code can write.
///
/// One table, read by both import passes — a file's own declarations and its
/// test file's — so that a symbol the emitter starts writing cannot be imported
/// in one and left undeclared in the other. The match is on whole words:
/// `Mutex` is a part of `AsyncMutex`, and matching any substring imported
/// std's `Mutex` into a file that only ever names tokio's.
///
pub(crate) const BASE_RUNTIME_SYMBOLS: [&str; 71] = [
    "Result", "Arc", "Weak", "Mutex", "MutexGuard",
    "RwLock", "RwLockReadGuard", "RwLockWriteGuard",
    "RefCell", "Ref", "RefMut", "ThreadLocal",
    // The closure that owns its captures, and the error `?` converts into.
    "OwnedClosure", "AnyhowError", "anyhow",
    // What an emitted `fromJson` answers with: serde_json::Error's stand-in,
    // the lossless reader and writer, and the two combinators a list or a map
    // reads through. `dropOwned` releases what a failed decode had already
    // built, and `OwnershipFatal` is what its `catch` has to rethrow.
    "JsonError", "serde_json", "jsonAll", "jsonMap", "dropOwned", "OwnershipFatal",
    // The logger every `tracing::` macro writes a call on.
    "tracing",
    // What a consuming match arm releases the payload it took no name for
    // with, and Rust's two eager boolean operators.
    "dropUnbound", "boolAnd", "boolOr",
    // R12: the hole an emitted file carries where the port has no lowering.
    "unsupported",
    // C1: the cell a `&mut` to a JavaScript VALUE is passed in.
    "BorrowMut",
    // R7: arithmetic on a fixed-width integer PANICS on overflow, as the
    // `debug_assertions = true` build this port mirrors does, and the four
    // families Rust offers for saying what should happen instead.
    "checkedAdd", "checkedSub", "checkedMul", "checkedDiv", "checkedRem",
    "wrappingAdd", "wrappingSub", "wrappingMul",
    "checkedAddOption", "checkedSubOption", "checkedMulOption",
    "saturatingAdd", "saturatingSub", "saturatingMul",
    "overflowingAdd", "overflowingSub", "overflowingMul",
    // The keyed containers a `HashMap`/`HashSet` becomes, and the hash a
    // derived key writes itself with.
    "HashMap", "HashSet", "keyHash",
    "AsyncMutex", "AsyncMutexGuard",
    "AsyncRwLock", "AsyncRwLockReadGuard", "AsyncRwLockWriteGuard",
    "Notify", "Notified", "TryLockError",
    "JoinHandle", "JoinError", "Elapsed",
    "tokio", "oneshot", "mpsc", "select", "spawn", "spawn_local", "yield_now",
    "sleep", "timeout",
    // The channel ends, which `mpsc::channel` hands back and a dispatcher names.
    "Sender", "UnboundedSender", "Receiver", "UnboundedReceiver",
];

pub fn generate_ts(reg: &TypeRegistry, file: &RustFile, rust_crate_path: &str) -> String {
    generate_ts_inner(reg, file, rust_crate_path, None)
}

fn generate_ts_inner(reg: &TypeRegistry, file: &RustFile, rust_crate_path: &str, config: Option<&crate::config::Config>) -> String {
    let mut out = String::new();

    // Line 1: MIRRORS annotation
    out.push_str(&format!("// MIRRORS: ankurah/{}\n", rust_crate_path));

    // Build FQN prefix from crate_path
    let fqn_prefix = crate_path_to_fqn_prefix(rust_crate_path);

    // Identify provided types and their import modules
    let mut provided_by_module: HashMap<String, Vec<String>> = HashMap::new();
    let mut provided_set: HashSet<String> = HashSet::new();

    for s in &file.structs {
        let fqn = format!("{}::{}", fqn_prefix, s.name);
        if let Some(module) = config.and_then(|c| c.provided_import_module(&fqn)) {
            provided_by_module.entry(module).or_default().push(s.name.clone());
            provided_set.insert(s.name.clone());
        }
    }
    for e in &file.enums {
        let fqn = format!("{}::{}", fqn_prefix, e.name);
        if let Some(module) = config.and_then(|c| c.provided_import_module(&fqn)) {
            provided_by_module.entry(module).or_default().push(e.name.clone());
            provided_set.insert(e.name.clone());
        }
    }

    // Collect local type names (including provided — keeps import resolution correct)
    let mut local_types: HashSet<String> = HashSet::new();
    for s in &file.structs { local_types.insert(s.name.clone()); }
    for e in &file.enums { local_types.insert(e.name.clone()); }
    for t in &file.traits { local_types.insert(t.name.clone()); }

    // Base imports (@ankurah/base) — only for non-provided types
    let mut base_imports: Vec<&str> = Vec::new();
    let has_non_provided_structs = file.structs.iter().any(|s| !provided_set.contains(&s.name));
    let has_non_provided_enums = file.enums.iter().any(|e| !provided_set.contains(&e.name));
    if has_non_provided_structs { base_imports.push("Struct"); }
    if has_non_provided_enums { base_imports.push("Enum"); }
    for imp in &file.impls {
        if imp.trait_name().as_deref() == Some("Drop") && !base_imports.contains(&"Drop") {
            base_imports.push("Drop");
        }
    }
    // What the file will actually contain. The import list is read off it.
    let emitted = generate_declarations(reg, file, &provided_set, None);

    // Auto-detect base types used in fields, return types, and method bodies
    let mut all_type_refs = String::new();
    for s in &file.structs {
        if provided_set.contains(&s.name) { continue; }
        for f in &s.fields { all_type_refs.push_str(&f.ts_ty(reg)); all_type_refs.push(' '); }
    }
    for e in &file.enums {
        if provided_set.contains(&e.name) { continue; }
        for v in &e.variants { for f in &v.fields { all_type_refs.push_str(&f.ts_ty(reg)); all_type_refs.push(' '); } }
    }
    for imp in &file.impls {
        if provided_set.contains(&imp.target_type) { continue; }
        for m in &imp.methods {
            all_type_refs.push_str(&m.return_type); all_type_refs.push(' ');
            for p in &m.params { all_type_refs.push_str(&p.ty); all_type_refs.push(' '); }
            if let Some(b) = &m.body_ts { all_type_refs.push_str(b); all_type_refs.push(' '); }
        }
    }
    for f in &file.functions {
        all_type_refs.push_str(&f.return_type); all_type_refs.push(' ');
        for p in &f.params { all_type_refs.push_str(&p.ty); all_type_refs.push(' '); }
        if let Some(b) = &f.body_ts { all_type_refs.push_str(b); all_type_refs.push(' '); }
    }
    for decl in &file.module_decls {
        all_type_refs.push_str(decl); all_type_refs.push(' ');
    }
    let base_runtime_types = BASE_RUNTIME_SYMBOLS;
    for ty in &base_runtime_types {
        // Read the emitted text, not the types the file mentions: a body that
        // was generated and then not emitted used to pull in an import nothing
        // in the file uses. Don't import if the file defines its own type with
        // the same name. The match is on whole words: `Mutex` is a part of
        // `AsyncMutex`, and matching on any substring imported std's `Mutex`
        // for a file that only ever names tokio's.
        if mentions(&emitted, ty) && !base_imports.contains(ty) && !local_types.contains(*ty) {
            base_imports.push(ty);
        }
    }
    // The cascade, which the ownership emission calls to release a plain
    // JavaScript value that owns what is inside it — an array of entities, a
    // map of them. It is a function rather than a type, so it is looked for by
    // the call the emitter writes.
    if emitted.contains("dropOwned(") && !base_imports.contains(&"dropOwned") {
        base_imports.push("dropOwned");
    }
    if !base_imports.is_empty() {
        out.push_str(&format!("import {{ {} }} from '@ankurah/base';\n", base_imports.join(", ")));
    }

    // Provided type imports
    let mut sorted_provided_modules: Vec<&String> = provided_by_module.keys().collect();
    sorted_provided_modules.sort();
    for module in &sorted_provided_modules {
        let mut types = provided_by_module[*module].clone();
        types.sort();
        out.push_str(&format!("import {{ {} }} from '{}';\n", types.join(", "), module));
    }

    // Cross-crate imports
    let mut cross_crate_imports: HashMap<String, Vec<String>> = HashMap::new();
    for u in &file.uses {
        if let Some((package, symbols)) = imports::resolve_use_import(&u.path) {
            cross_crate_imports.entry(package).or_default().extend(symbols);
        }
    }
    let mut sorted_packages: Vec<&String> = cross_crate_imports.keys().collect();
    sorted_packages.sort();
    for package in &sorted_packages {
        let mut symbols = cross_crate_imports[*package].clone();
        symbols.sort();
        symbols.dedup();
        let symbols: Vec<String> = symbols.into_iter()
            .filter(|s| !imports::is_primitive_or_base_type(s)
                && s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
            .collect();
        if !symbols.is_empty() {
            out.push_str(&format!("import {{ {} }} from '{}';\n", symbols.join(", "), package));
        }
    }

    // Bincode imports — only for non-provided types
    let needs_bincode = file.structs.iter().any(|s|
            !provided_set.contains(&s.name) && crate::bincode_module::has_serde_derive(&s.derives))
        || file.enums.iter().any(|e|
            !provided_set.contains(&e.name) && crate::bincode_module::has_serde_derive(&e.derives));
    if needs_bincode {
        out.push_str("import { BincodeReader, BincodeWriter } from './codec';\n");
    }

    // Remaining unresolved external type references — scan everything
    let mut referenced_types: HashSet<String> = HashSet::new();
    for s in &file.structs {
        if provided_set.contains(&s.name) { continue; }
        for f in &s.fields { imports::collect_type_refs(&f.ts_ty(reg), &mut referenced_types); }
    }
    for e in &file.enums {
        if provided_set.contains(&e.name) { continue; }
        for v in &e.variants {
            for f in &v.fields { imports::collect_type_refs(&f.ts_ty(reg), &mut referenced_types); }
        }
    }
    // Also scan function/method signatures and bodies
    for imp in &file.impls {
        if provided_set.contains(&imp.target_type) { continue; }
        for m in &imp.methods {
            imports::collect_type_refs(&m.return_type, &mut referenced_types);
            for p in &m.params { imports::collect_type_refs(&p.ty, &mut referenced_types); }
            if let Some(b) = &m.body_ts { imports::collect_type_refs(b, &mut referenced_types); }
        }
    }
    for f in &file.functions {
        imports::collect_type_refs(&f.return_type, &mut referenced_types);
        for p in &f.params { imports::collect_type_refs(&p.ty, &mut referenced_types); }
        if let Some(b) = &f.body_ts { imports::collect_type_refs(b, &mut referenced_types); }
    }
    // Module-level declarations
    for decl in &file.module_decls {
        imports::collect_type_refs(decl, &mut referenced_types);
    }

    let mut imported_symbols: HashSet<String> = HashSet::new();
    for symbols in cross_crate_imports.values() {
        for s in symbols { imported_symbols.insert(s.clone()); }
    }

    let external_types: Vec<&String> = referenced_types.iter()
        .filter(|t| !local_types.contains(*t) && !imports::is_primitive_or_base_type(t)
            && !imported_symbols.contains(*t))
        .collect();

    if !external_types.is_empty() {
        let mut sorted: Vec<&&String> = external_types.iter().collect();
        sorted.sort();
        out.push_str(&format!("// TODO imports: {}\n",
            sorted.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ")));
    }

    // Re-export provided types
    if !provided_set.is_empty() {
        let mut names: Vec<&String> = provided_set.iter().collect();
        names.sort();
        out.push_str(&format!("export {{ {} }};\n", names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    }

    // `pub use auth::*;` is what makes a crate's names reachable from its root,
    // and the port's `index.ts` has to say the same or the package exports
    // nothing at all. Without it the emitted index was a header and a blank
    // line, and every name the package is supposed to offer — `QueryId` among
    // them — was reachable only by importing the module it was declared in.
    for line in public_reexports(reg, file, rust_crate_path, config) {
        out.push_str(&line);
    }

    out.push('\n');

    out.push_str(&emitted);
    out
}

/// What this file re-exports from the modules under it, as the `export` lines
/// the port writes.
///
/// `pub use auth::*;` and `pub use subscription::QueryId;` are what make a
/// crate's names reachable from its root, and the port's `index.ts` has to say
/// the same or the package offers nothing. Without them the emitted index was a
/// header and a blank line, and `QueryId` — re-exported by name — was reachable
/// only by importing the module it was declared in.
///
/// Only a module this file declares: `pub use serde::*` is another crate's
/// business, and the cross-crate import machinery writes that where it is used.
fn public_reexports(
    reg: &TypeRegistry,
    file: &RustFile,
    corpus_path: &str,
    config: Option<&crate::config::Config>,
) -> Vec<String> {
    let Some(module) = reg.modules().lookup_file(&file.path) else {
        return Vec::new();
    };
    let children = &reg.modules().get(module).children;
    let mut out: Vec<String> = Vec::new();
    // `pub mod ast;` is how `ankql::ast::Expr` becomes reachable from outside
    // the crate. TypeScript has no nested module namespace to mirror, and the
    // port's own hand-written indexes settled the convention long ago: a public
    // child module is re-exported whole. Without this the emitted `index.ts`
    // for a crate whose root is nothing but `pub mod` lines — ankql's — was a
    // header and a blank line, and the package exported nothing at all.
    let mut whole_modules: Vec<String> = Vec::new();
    // Which module each `export * from './m'` flattens, so the ambiguity pass
    // below can ask what names it brings. A `[[provided]]` module is somebody's
    // hand-written TypeScript and the registry does not hold its names, so it
    // is left out of that question.
    let mut star_modules: Vec<(String, crate::registry::ModuleId)> = Vec::new();
    for (name, vis) in &file.mod_decls {
        if *vis != crate::types::VisInfo::Public {
            continue;
        }
        let provided = provided_child_module(corpus_path, name, config);
        let target = provided.clone().unwrap_or_else(|| child_module(&file.path, name));
        let line = format!("export * from '{}';\n", target);
        if !out.contains(&line) {
            out.push(line);
            if provided.is_none() {
                if let Some(child) = children.get(name) {
                    star_modules.push((target.clone(), *child));
                }
            }
            whole_modules.push(target);
        }
    }
    // A TypeScript-only module the port adds beside this crate.
    if let Some(cfg) = config {
        for extra in cfg.extra_exports_in(corpus_path) {
            let line = format!("export * from './{}';\n", extra.module);
            if !out.contains(&line) {
                out.push(line);
            }
        }
    }
    // `pub use ankurah_proto as proto;` gives the crate a LOCAL name, and the
    // line below it — `pub use proto::EntityId;` — reaches the crate through
    // that name. Without the map the second line names nothing and `EntityId`
    // was simply absent from the facade's surface.
    let mut crate_aliases: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for u in &file.uses {
        for binding in &u.bindings {
            if let (Some(local), [one]) = (&binding.local, &binding.path[..]) {
                if reg.sibling_crate(one).is_some() {
                    crate_aliases.insert(local.as_str(), one.as_str());
                }
            }
        }
    }
    for u in &file.uses {
        if u.vis != crate::types::VisInfo::Public {
            continue;
        }
        for binding in &u.bindings {
            // `pub use ankurah_proto as proto;` and `pub use proto::EntityId;`
            // name ANOTHER CRATE, and a crate is a package here, not a file
            // beside this one. The registry keeps a sibling's root among this
            // module's children, so asking `children` alone wrote
            // `export { proto } from './ankurah_proto'` — 29 broken module
            // specifiers in the facade's index, which is every own-file error
            // that package had.
            let head = binding.path.first().map(String::as_str).unwrap_or_default();
            let head = crate_aliases.get(head).copied().unwrap_or(head);
            if let Some(package) = sibling_package(reg, head) {
                // `pub use ankurah_core::{changes, entity, ..}` re-exports
                // another crate's MODULES. The port flattens a crate's modules
                // into its package surface — `export * from './changes'` — so
                // there is no `changes` name on the other side to re-export,
                // and writing one names nothing.
                if let (Some(local), [_, name]) = (&binding.local, &binding.path[..]) {
                    if reg
                        .sibling_crate(head)
                        .is_some_and(|root| reg.modules().get(root).children.contains_key(name))
                    {
                        crate::diag::pending::park_at(
                            0,
                            0,
                            format!(
                                "`{}` re-exports `{}`, which is a MODULE of that crate, and the \
                                 port flattens a crate's modules into its package surface, so \
                                 there is no name to re-export",
                                package, local
                            ),
                        );
                        continue;
                    }
                }
                let line = match (&binding.local, &binding.path[..]) {
                    // `pub use ankql;` / `pub use ankurah_core as core;` —
                    // the whole crate under one name.
                    (Some(local), [_one]) => {
                        format!("export * as {} from '{}';\n", local, package)
                    }
                    // `pub use proto::EntityId;` — one name out of it.
                    (Some(local), [_, ..]) => {
                        format!("export {{ {} }} from '{}';\n", local, package)
                    }
                    // `pub use ankurah_derive::*;`
                    (None, _) => format!("export * from '{}';\n", package),
                    // A binding with a local name and no path is not a shape
                    // `use` produces.
                    (Some(_), []) => continue,
                };
                if !out.contains(&line) {
                    out.push(line);
                }
                continue;
            }
            let line = match (&binding.local, &binding.path[..]) {
                (None, [name]) if children.contains_key(name) => {
                    let provided = provided_child_module(corpus_path, name, config);
                    let target = provided.clone().unwrap_or_else(|| child_module(&file.path, name));
                    if provided.is_none() && !star_modules.iter().any(|(t, _)| *t == target) {
                        star_modules.push((target.clone(), children[name]));
                    }
                    format!("export * from '{}';\n", target)
                }
                (Some(local), [name, ..]) if children.contains_key(name) => {
                    let target = provided_child_module(corpus_path, name, config)
                        .unwrap_or_else(|| child_module(&file.path, name));
                    // `pub mod broadcast;` beside `pub use broadcast::BroadcastId;`
                    // is two true statements about one module, and the star
                    // export already carries the name.
                    if whole_modules.contains(&target) {
                        continue;
                    }
                    format!("export {{ {} }} from '{}';\n", local, target)
                }
                _ => continue,
            };
            if !out.contains(&line) {
                out.push(line);
            }
        }
    }
    out.extend(disambiguate_stars(reg, file, &star_modules));
    out
}

/// The explicit re-exports that keep a name two star exports both offer.
///
/// `export * from './broadcast'` and `export * from './signal'` both offering
/// `ListenerGuard` means JavaScript exports it from NEITHER — an ambiguous star
/// export is dropped silently, so `@ankurah/signals` had no `ListenerGuard` at
/// all in either spelling. An explicit export shadows every star export of that
/// name, so writing one settles it: the module Rust itself reaches unqualified
/// from the crate root (`pub use signal::*`) keeps the bare name, and every
/// other module keeps its own under a module qualifier. Where Rust reaches
/// none of them unqualified there is no bare name to award, so all of them are
/// qualified and the report says the bare spelling is gone.
fn disambiguate_stars(
    reg: &TypeRegistry,
    file: &RustFile,
    star_modules: &[(String, crate::registry::ModuleId)],
) -> Vec<String> {
    use crate::codegen::surface;
    if star_modules.len() < 2 {
        return Vec::new();
    }
    let surfaces: Vec<(String, std::collections::BTreeMap<String, crate::registry::Def>)> = star_modules
        .iter()
        .map(|(specifier, id)| (specifier.clone(), surface::star_surface(reg, *id)))
        .collect();

    // Which specifier, if any, Rust reaches this name through UNQUALIFIED from
    // the crate root. `pub mod broadcast;` is not such a reach: it makes the
    // name reachable only as `broadcast::ListenerGuard`.
    let mut unqualified: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for u in &file.uses {
        if u.vis != crate::types::VisInfo::Public {
            continue;
        }
        for binding in &u.bindings {
            let Some(head) = binding.path.first() else { continue };
            let Some((specifier, _)) = star_modules
                .iter()
                .find(|(t, _)| t.rsplit('/').next() == Some(head.as_str()))
            else {
                continue;
            };
            match (&binding.local, &binding.path[..]) {
                // `pub use signal::ListenerGuard;` — this one name.
                (Some(local), [_, ..]) => {
                    unqualified.insert(local.clone(), specifier.clone());
                }
                // `pub use signal::*;` — everything the module offers.
                (None, [_]) => {
                    for name in surface::star_surface(reg, star_modules.iter().find(|(t, _)| t == specifier).unwrap().1).into_keys() {
                        unqualified.entry(name).or_insert_with(|| specifier.clone());
                    }
                }
                _ => {}
            }
        }
    }

    let mut lines = Vec::new();
    for found in surface::ambiguities(&surfaces, |name| unqualified.get(name).cloned()) {
        for specifier in &found.modules {
            if found.bare.as_deref() == Some(specifier.as_str()) {
                lines.push(format!("export {{ {} }} from '{}';\n", found.name, specifier));
            } else {
                lines.push(format!(
                    "export {{ {} as {} }} from '{}';\n",
                    found.name,
                    found.alias(specifier),
                    specifier
                ));
            }
        }
        let where_bare = match &found.bare {
            Some(m) => format!("`{}` keeps the bare name because the crate root reaches it there unqualified", m),
            None => format!(
                "the crate root reaches none of them unqualified, so `{}` is not exported bare at all",
                found.name
            ),
        };
        crate::diag::pending::park_at(
            0,
            0,
            format!(
                "`{}` is declared in {}, and the port flattens a crate's modules into one package \
                 surface, where two star exports of one name export it from neither. Each keeps \
                 its own name qualified by its module ({}); {}",
                found.name,
                found.modules.join(" and "),
                found
                    .modules
                    .iter()
                    .filter(|m| found.bare.as_deref() != Some(m.as_str()))
                    .map(|m| found.alias(m))
                    .collect::<Vec<_>>()
                    .join(", "),
                where_bare
            ),
        );
    }
    lines
}

/// The package a `pub use` of another crate re-exports from, where the head of
/// the path names one.
///
/// A crate the port does not carry — `ankurah_derive`, whose macros are
/// expanded away — has no package to name, and the re-export is reported rather
/// than written against a specifier nothing resolves.
fn sibling_package(reg: &TypeRegistry, head: &str) -> Option<String> {
    reg.sibling_crate(head)?;
    match crate::name_map::map_crate_to_package(head) {
        Some(package) => Some(package.to_string()),
        None => None,
    }
}

/// Where a hand-written child module sits, when the TypeScript it is called is
/// not what the Rust module is called. A `[[provided]]` entry names both, so a
/// re-export of `mod connection;` reaches `connection.provided.ts` where that is
/// what somebody wrote.
fn provided_child_module(
    corpus_path: &str,
    child: &str,
    config: Option<&crate::config::Config>,
) -> Option<String> {
    let cfg = config?;
    // The parent's own directory, as a corpus path: `ankql/src/lib.rs` puts its
    // children at `ankql/src/<child>.rs`.
    let dir = corpus_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let stem = corpus_path
        .rsplit('/')
        .next()
        .unwrap_or(corpus_path)
        .trim_end_matches(".rs");
    let candidate = match (dir, stem) {
        ("", "lib") | ("", "mod") => format!("{child}.rs"),
        ("", other) => format!("{other}/{child}.rs"),
        (dir, "lib") | (dir, "mod") => format!("{dir}/{child}.rs"),
        (dir, other) => format!("{dir}/{other}/{child}.rs"),
    };
    let provided = cfg.provided_module(&candidate)?;
    // `module` is relative to the package's src/; this file imports it relative
    // to itself, and everything that re-exports a child is a module index.
    let last = provided.module.rsplit('/').next().unwrap_or(&provided.module);
    Some(match (dir, stem) {
        (_, "lib") | (_, "mod") => format!("./{last}"),
        (_, other) => format!("./{other}/{last}"),
    })
}

/// Where a child module's file sits, as this file would import it.
///
/// A crate root — `lib.rs`, emitted as `index.ts` — has its children beside it:
/// `./auth`. Any other module keeps its children in a directory named after
/// itself, so `signal.rs`'s `calculated` is at `./signal/calculated`. Writing
/// `./calculated` from `signal.ts` named a file that is not there.
fn child_module(file_path: &str, child: &str) -> String {
    let stem = file_path
        .rsplit('/')
        .next()
        .unwrap_or(file_path)
        .trim_end_matches(".rs");
    match stem {
        "lib" | "mod" => format!("./{}", child),
        other => format!("./{}/{}", other, child),
    }
}

/// Everything the file declares, as TypeScript.
///
/// This runs before the import list is built, because what the file imports is
/// decided by what it emits: a body that was generated and then not emitted —
/// an `impl Display` for a type another file declares — used to pull in an
/// import nothing in the output names.
fn generate_declarations(
    reg: &TypeRegistry,
    file: &RustFile,
    provided_set: &HashSet<String>,
    // Which module this file's declarations belong to. `None` looks it up from
    // the file's path, which is what a real file has; an INLINE module has no
    // path of its own, and looking one up for it answered the crate root — so
    // every impl in a test module was read as "declared in another module" and
    // came out as a free function.
    module: Option<crate::registry::ModuleId>,
) -> String {
    let mut out = String::new();
    // Organize impl blocks
    let mut inherent_methods: HashMap<String, Vec<&FnInfo>> = HashMap::new();
    let mut trait_impls: HashMap<String, Vec<(&str, &[String])>> = HashMap::new();
    let mut trait_methods: HashMap<String, Vec<(&str, &[String], &FnInfo, &[String])>> = HashMap::new();

    // The trait an impl block names lives on it as the `syn::Path` the source
    // wrote. Emission needs the TypeScript spelling of the name and of each
    // argument, derived once here so the maps below can borrow it.
    // Two spellings of one impl's trait arguments, and they are not
    // interchangeable: the `implements` clause needs the TypeScript type
    // (`GetReadCell<T | null>`), and the method NAME needs the path as written
    // (`From<bincode::Error>`), which is what tells two conversions apart.
    let impl_traits: Vec<(Option<String>, Vec<String>, Vec<String>)> = file
        .impls
        .iter()
        .map(|i| (i.trait_name(), i.trait_type_args(), i.trait_type_arg_paths()))
        .collect();

    // An impl whose self type has no emitted class contributes module-level
    // functions instead of methods, and its methods must not also be hung on a
    // class named after its target — there is none.
    let here = module.or_else(|| reg.modules().lookup_file(&file.path));
    let free: Vec<crate::emit_impls::FreeFn> = match here {
        Some(module) => crate::emit_impls::free_functions(reg, module, file),
        None => Vec::new(),
    };
    // A trait this file declares carries the function that picks among its
    // impls at run time, for the calls that dispatch through a bound the engine
    // cannot close.
    let dispatchers: Vec<crate::emit_impls::Dispatcher> = match here {
        Some(module) => crate::emit_impls::dispatchers(reg, module, file),
        None => Vec::new(),
    };
    let on_a_class = |imp: &ImplInfo| match here {
        Some(module) => crate::emit_impls::impl_has_class(reg, module, imp),
        None => true,
    };

    for (imp, (trait_name, type_args, written_args)) in file.impls.iter().zip(&impl_traits) {
        if !on_a_class(imp) {
            continue;
        }
        if let Some(trait_name) = trait_name {
            trait_impls.entry(imp.target_type.clone()).or_default().push((trait_name.as_str(), type_args.as_slice()));
            for method in &imp.methods {
                trait_methods.entry(imp.target_type.clone())
                    .or_default()
                    .push((trait_name.as_str(), written_args.as_slice(), method, imp.type_params.as_slice()));
            }
        } else {
            inherent_methods.entry(imp.target_type.clone()).or_default().extend(imp.methods.iter());
        }
    }

    // Collect generic bounds from all impl blocks for each type.
    // Merges inline bounds + where clause bounds across all impls.
    let mut impl_bounds: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    for imp in &file.impls {
        let bounds = imp.generic_bounds();
        if !bounds.is_empty() {
            let type_bounds = impl_bounds.entry(imp.target_type.clone()).or_default();
            for (param, bounds) in &bounds {
                let existing = type_bounds.entry(param.clone()).or_default();
                for b in bounds {
                    if !existing.contains(b) {
                        existing.push(b.clone());
                    }
                }
            }
        }
    }

    // Emit items (skip provided types — already imported and re-exported above)
    for s in &file.structs {
        if provided_set.contains(&s.name) {
            continue;
        }
        emit::emit_struct(&mut out, reg, here, s, &inherent_methods, &trait_impls, &trait_methods, impl_bounds.get(&s.name), &file.assigned_fields);
    }
    for e in &file.enums {
        if provided_set.contains(&e.name) {
            continue;
        }
        emit::emit_enum(&mut out, reg, here, e, &inherent_methods, &trait_impls, &trait_methods);
    }
    for t in &file.traits {
        emit::emit_trait(&mut out, t);
    }
    for f in &file.functions {
        // A test module's functions are written inside the `describe` of the
        // `.test.ts`, not as module-level functions beside its fixtures.
        if !f.is_test && !file.is_test_module {
            emit::emit_function(&mut out, f);
        }
    }
    for t in &file.type_aliases {
        let export = if t.is_pub { "export " } else { "" };
        out.push_str(&format!("{}type {} = {};\n\n", export, t.name, t.ty));
    }
    for c in &file.consts {
        // Skip consts that have a module_decl (e.g., thread_local constants)
        let has_decl = file.module_decls.iter().any(|d| d.contains(&c.name));
        if has_decl { continue; }
        let export = if c.is_pub { "export " } else { "" };
        // Rust's `static mut` is a global the program writes to; everything
        // else here is a value fixed at load.
        let keyword = if c.mutable { "let" } else { "const" };
        match &c.init_ts {
            Some(init) => out.push_str(&format!(
                "{}{} {}: {} = {};\n\n",
                export, keyword, c.name, c.ty, init
            )),
            None => out.push_str(&format!(
                "{}{} {}: {} = undefined as any; // TODO\n\n",
                export, keyword, c.name, c.ty
            )),
        }
    }

    // The impls with no class of their own, as the functions they become.
    for f in &free {
        out.push_str(&f.text);
    }

    // The run-time selection among a trait's impls, for the calls that cannot
    // name one.
    for d in &dispatchers {
        out.push_str(&d.text);
    }

    // Module-level declarations (thread_local, etc.)
    for decl in &file.module_decls {
        out.push_str(decl);
        out.push_str("\n\n");
    }

    out
}

pub fn generate_test_ts_with_imports(
    reg: &TypeRegistry,
    file: &RustFile,
    rust_crate_path: &str,
    type_to_file: &HashMap<String, String>,
    current_module: &str,
) -> Option<String> {
    if file.test_functions.is_empty() {
        return None;
    }

    let mut out = String::new();
    out.push_str(&format!("// MIRRORS: ankurah/{} (tests module)\n\n", rust_crate_path));
    out.push_str("import { describe, test, expect } from 'bun:test';\n");

    // Extract module name from crate path for describe block
    let module_name = rust_crate_path
        .rsplit('/')
        .next()
        .unwrap_or(rust_crate_path)
        .replace(".rs", "");

    // Import types from the parent module that are used in test bodies
    let mut available_types: HashSet<String> = HashSet::new();
    for s in &file.structs { available_types.insert(s.name.clone()); }
    for e in &file.enums { available_types.insert(e.name.clone()); }
    for t in &file.traits { available_types.insert(t.name.clone()); }

    // Collect all type references from test bodies
    let mut test_refs: HashSet<String> = HashSet::new();
    for f in file.test_functions.iter().chain(&file.test_helpers) {
        if let Some(body) = &f.body_ts {
            imports::collect_type_refs(body, &mut test_refs);
        }
    }

    // The fixtures the test module declares, written once here and used twice:
    // to decide the imports, and as the text emitted above the `describe`.
    let fixtures = match &file.test_module {
        Some(name) => file
            .inline_modules
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, sub)| {
                let id = reg
                    .modules()
                    .lookup_file(&file.path)
                    .and_then(|parent| reg.modules().get(parent).children.get(name).copied());
                generate_declarations(reg, sub, &HashSet::new(), id)
            })
            .unwrap_or_default(),
        None => String::new(),
    };

    // A fixture the test module declares is written into this file, so it is
    // not imported from anywhere: `import { TestEntity } from './tests'` named
    // a module that is not emitted at all.
    let declared_here: HashSet<String> = match &file.test_module {
        Some(name) => file
            .inline_modules
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, sub)| {
                sub.structs
                    .iter()
                    .map(|s| s.name.clone())
                    .chain(sub.enums.iter().map(|e| e.name.clone()))
                    .chain(sub.traits.iter().map(|t| t.name.clone()))
                    .collect()
            })
            .unwrap_or_default(),
        None => HashSet::new(),
    };
    test_refs.retain(|name| !declared_here.contains(name));

    // `use super::*` — which is how every test module in the corpus opens —
    // brings the WHOLE parent module into the test's scope: its functions and
    // its consts as well as its types. Only the types were importable, so
    // `selection/sql.test.ts` never imported `generateSelectionSql` and all
    // fourteen of its tests died on a `ReferenceError`.
    let mut from_parent: Vec<String> = test_refs
        .iter()
        .filter(|t| available_types.contains(*t))
        .cloned()
        .collect();
    let fixture_text = fixtures.clone();
    let bodies_and_fixtures: String = file
        .test_functions
        .iter()
        .chain(&file.test_helpers)
        .filter_map(|f| f.body_ts.as_deref())
        .chain(std::iter::once(fixture_text.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    for f in &file.functions {
        if f.is_test || declared_here.contains(&f.ts_name) {
            continue;
        }
        if names_word(&bodies_and_fixtures, &f.ts_name) && !from_parent.contains(&f.ts_name) {
            from_parent.push(f.ts_name.clone());
        }
    }
    for c in &file.consts {
        if declared_here.contains(&c.name) {
            continue;
        }
        if names_word(&bodies_and_fixtures, &c.name) && !from_parent.contains(&c.name) {
            from_parent.push(c.name.clone());
        }
    }
    if !from_parent.is_empty() {
        from_parent.sort();
        from_parent.dedup();
        out.push_str(&format!(
            "import {{ {} }} from './{}';\n",
            from_parent.join(", "),
            module_name
        ));
    }

    // Import base types (Arc, Mutex, RefCell, etc.)
    let base_runtime_types = BASE_RUNTIME_SYMBOLS;
    // The fixture declarations are part of this file too, and they name
    // `Struct`, `Enum` and whatever else the runtime supplies.
    let all_bodies: String = bodies_and_fixtures.clone();
    // Read the emitted bodies rather than the PascalCase names the type scan
    // found: `dropOwned` is a function and `oneshot` a namespace, and neither is
    // a type reference.
    let mut base_imports: Vec<&&str> = base_runtime_types.iter()
        .filter(|t| mentions(&all_bodies, t) && !available_types.contains(**t))
        .collect();
    let cascade = "dropOwned";
    if all_bodies.contains("dropOwned(") {
        base_imports.push(&cascade);
    }
    // The bases a fixture class extends. `Struct` and `Enum` are not in the
    // runtime-symbol table — the main file's import list adds them from what it
    // DECLARES, and a test file declares its fixtures the same way.
    let struct_base = "Struct";
    let enum_base = "Enum";
    if fixtures.contains("extends Struct") {
        base_imports.push(&struct_base);
    }
    if fixtures.contains("extends Enum<") {
        base_imports.push(&enum_base);
    }
    if !base_imports.is_empty() {
        let mut sorted = base_imports;
        sorted.sort();
        sorted.dedup();
        out.push_str(&format!("import {{ {} }} from '@ankurah/base';\n",
            sorted.iter().map(|s| **s).collect::<Vec<_>>().join(", ")));
    }

    // A module-level function the suite calls — `parseSelection`,
    // `generateSelectionSql` — is a name, not a type, so the PascalCase scan
    // above passes over it. A test-module helper is where these turn up:
    // ankql's `nullify_columns` calls two of them.
    let free_names: std::collections::HashSet<String> = type_to_file
        .keys()
        .filter(|name| name.chars().next().is_some_and(|c| c.is_lowercase()))
        .cloned()
        .collect();
    let mut test_refs = test_refs;
    if !free_names.is_empty() {
        imports::collect_named_refs(&all_bodies, &free_names, &mut test_refs);
    }

    // Cross-file imports from the same crate (using type_to_file map)
    let mut cross_file_imports: HashMap<String, Vec<String>> = HashMap::new();
    for type_name in &test_refs {
        if available_types.contains(type_name) || imports::is_primitive_or_base_type(type_name) {
            continue;
        }
        if base_runtime_types.contains(&type_name.as_str()) {
            continue; // Already imported from @ankurah/base
        }
        if let Some(source_module) = type_to_file.get(type_name) {
            // Compute the test file's module path (test is in same dir as parent)
            let test_module = current_module;
            if source_module != test_module {
                let import_path = relative_import_path(test_module, source_module);
                cross_file_imports.entry(import_path)
                    .or_default()
                    .push(type_name.clone());
            }
        }
    }
    let mut sorted_cross: Vec<&String> = cross_file_imports.keys().collect();
    sorted_cross.sort();
    for module in sorted_cross {
        let mut types = cross_file_imports[module].clone();
        types.sort();
        out.push_str(&format!("import {{ {} }} from '{}';\n", types.join(", "), module));
    }

    // Bincode imports if test bodies reference them
    let all_test_body: String = file.test_functions.iter().chain(&file.test_helpers)
        .filter_map(|f| f.body_ts.as_deref())
        .collect::<Vec<_>>().join(" ");
    if all_test_body.contains("BincodeWriter") || all_test_body.contains("BincodeReader") {
        out.push_str("import { BincodeWriter, BincodeReader } from './codec';\n");
    }
    out.push('\n');

    // The fixtures the test module declares — a struct, an impl on it, a
    // `const` — written before the `describe` that names them. `mod tests` is
    // ordinary Rust and every non-`fn` item in it used to be dropped.
    if !fixtures.trim().is_empty() {
        out.push_str(&fixtures);
    }

    out.push_str(&format!("describe('{} unit tests', () => {{\n", module_name));

    // The helpers first: every test that calls one is written below it.
    for f in &file.test_helpers {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{}: {}", crate::name_map::to_camel_case(&p.name), p.ty))
            .collect();
        let ret = if f.return_type.is_empty() { "void".to_string() } else { f.return_type.clone() };
        let body = match &f.body_ts {
            Some(body) => body
                .lines()
                .map(|line| if line.is_empty() { String::new() } else { format!("    {}", line) })
                .collect::<Vec<_>>()
                .join("\n"),
            None => "    throw new Error('TODO');".to_string(),
        };
        out.push_str(&format!(
            "  {}function {}({}): {} {{\n{}\n  }}\n\n",
            if f.is_async { "async " } else { "" },
            f.ts_name,
            params.join(", "),
            if f.is_async { format!("Promise<{}>", ret) } else { ret },
            body
        ));
    }

    for f in &file.test_functions {
        let test_name = &f.name;
        let async_kw = if f.is_async { "async " } else { "" };
        let body = if let Some(body_ts) = &f.body_ts {
            body_ts.lines()
                .map(|line| if line.is_empty() { String::new() } else { format!("    {}", line) })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "    throw new Error('TODO');".to_string()
        };
        // `#[test] fn t() -> anyhow::Result<()>` fails when it answers `Err`,
        // and Rust's harness is what reads that answer. A bun test callback
        // has no such reader: it returns the `Result` to nobody, the failure
        // is swallowed and the test passes — twenty callbacks in the emitted
        // corpus ended `return Result.Ok([])`, and every `?` in one returned an
        // `Err` that meant nothing. So the body becomes a function and the
        // callback unwraps what it answers: `unwrap()` consumes the `Result`,
        // so nothing leaks, and throws on `Err`, which is what fails a test.
        let returns_a_result = f
            .return_type
            .split('<')
            .next()
            .is_some_and(|head| head.trim() == "Result");
        if returns_a_result {
            let inner = body
                .lines()
                .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
                .collect::<Vec<_>>()
                .join("\n");
            out.push_str(&format!(
                "  test('{}', {}() => {{\n    {}({}() => {{\n{}\n    }})().unwrap();\n  }});\n\n",
                test_name,
                async_kw,
                if f.is_async { "await " } else { "" },
                async_kw,
                inner
            ));
            continue;
        }
        out.push_str(&format!("  test('{}', {}() => {{\n{}\n  }});\n\n",
            test_name, async_kw, body));
    }

    out.push_str("});\n");

    Some(out)
}

/// Convert crate path to FQN prefix
/// "proto/src/error.rs" → "ankurah_proto::error"
/// "core/src/entity.rs" → "ankurah_core::entity"
/// "ankql/src/ast.rs" → "ankql::ast"
fn crate_path_to_fqn_prefix(crate_path: &str) -> String {
    // crate_path is like "proto/src/error.rs"
    let parts: Vec<&str> = crate_path.split('/').collect();
    if parts.len() < 3 {
        return crate_path.replace('/', "::").replace(".rs", "");
    }

    let crate_name = parts[0];
    // Map crate dir name to Rust crate name
    let rust_crate = match crate_name {
        "proto" => "ankurah_proto",
        "core" => "ankurah_core",
        "signals" => "ankurah_signals",
        "ankql" => "ankql",
        "storage-common" | "storage/common" => "ankurah_storage_common",
        "storage-sqlite" | "storage/sqlite" => "ankurah_storage_sqlite",
        "storage-postgres" | "storage/postgres" => "ankurah_storage_postgres",
        "storage-indexeddb" | "storage/indexeddb-wasm" => "ankurah_storage_indexeddb_wasm",
        other => other,
    };

    // Everything after "src/" is the module path
    let module_path = parts[2..].join("::")
        .replace(".rs", "")
        .replace("mod", "")
        .replace("lib", "");

    if module_path.is_empty() || module_path == "::" {
        rust_crate.to_string()
    } else {
        format!("{}::{}", rust_crate, module_path.trim_matches(':'))
    }
}

/// Compute relative import path from `current_module` to `target_module`.
///
/// Both are TS module specifiers like `./signal/calculated` or `./broadcast`.
/// Non-relative paths (e.g., `@ankurah/proto`) are returned unchanged.
///
/// Examples:
///   ("./signal/calculated", "./broadcast")         → "../broadcast"
///   ("./signal/calculated", "./signal/map")         → "./map"
///   ("./broadcast", "./signal/calculated")          → "./signal/calculated"
///   ("./observer/callback_observer", "./broadcast")  → "../broadcast"
fn relative_import_path(current_module: &str, target_module: &str) -> String {
    // Only adjust paths that are relative (start with "./")
    if !target_module.starts_with("./") || !current_module.starts_with("./") {
        return target_module.to_string();
    }

    // Strip leading "./" to get bare paths like "signal/calculated" or "broadcast"
    let current = &current_module[2..];
    let target = &target_module[2..];

    // Get directory of the current module (everything before last '/')
    let current_dir = match current.rfind('/') {
        Some(pos) => &current[..pos],
        None => "", // current module is at root level
    };

    // If both are at root level, no adjustment needed
    if current_dir.is_empty() {
        return target_module.to_string();
    }

    // Get directory and filename of the target module
    let (target_dir, target_file) = match target.rfind('/') {
        Some(pos) => (&target[..pos], &target[pos + 1..]),
        None => ("", target),
    };

    // Split into path components
    let current_parts: Vec<&str> = current_dir.split('/').collect();
    let target_parts: Vec<&str> = if target_dir.is_empty() {
        Vec::new()
    } else {
        target_dir.split('/').collect()
    };

    // Find common prefix length
    let common = current_parts.iter().zip(target_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Number of ".." needed to go up from current dir to common ancestor
    let ups = current_parts.len() - common;

    // Remaining target path components after common prefix
    let remaining_dirs = &target_parts[common..];

    // Build the relative path
    let mut parts: Vec<&str> = Vec::new();
    if ups == 0 {
        parts.push(".");
    } else {
        for _ in 0..ups {
            parts.push("..");
        }
    }
    for dir in remaining_dirs {
        parts.push(dir);
    }
    parts.push(target_file);

    parts.join("/")
}

/// Does this text name `word` as a word of its own?
///
/// The import scan reads emitted text rather than a symbol table, so a
/// substring match imported `Mutex` for a file that only ever wrote
/// `AsyncMutex`, and `Ref` for one that only wrote `RefCell`.
fn mentions(text: &str, word: &str) -> bool {
    let boundary = |c: char| !(c.is_alphanumeric() || c == '_' || c == '$');
    let mut from = 0;
    while let Some(at) = text[from..].find(word) {
        let start = from + at;
        let end = start + word.len();
        // A member read is not a name the file has to import: `tokio.mpsc`
        // needs `tokio`, and nothing else.
        let previous = text[..start].chars().next_back();
        let before = previous.is_none_or(boundary) && previous != Some('.');
        let after = text[end..].chars().next().is_none_or(boundary);
        if before && after {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Does this text name `word` as a whole identifier?
///
/// The import lists are built by looking for a name in emitted text, and a
/// substring match imported `Mutex` into a file that only ever wrote
/// `AsyncMutex`.
fn names_word(text: &str, word: &str) -> bool {
    let is_part = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let mut from = 0usize;
    while let Some(at) = text[from..].find(word) {
        let start = from + at;
        let end = start + word.len();
        let before = text[..start].chars().next_back().is_some_and(is_part);
        let after = text[end..].chars().next().is_some_and(is_part);
        if !before && !after {
            return true;
        }
        from = end;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extra(pairs: &[(&str, &[&str])]) -> std::collections::BTreeMap<String, BTreeSet<String>> {
        pairs
            .iter()
            .map(|(m, ns)| (m.to_string(), ns.iter().map(|n| n.to_string()).collect()))
            .collect()
    }

    /// `#[test] fn t() -> anyhow::Result<()>` FAILS when it answers `Err`, and
    /// Rust's harness is what reads that answer. A bun test callback has no
    /// such reader: it returns the `Result` to nobody and the failure is
    /// swallowed. Twenty emitted callbacks ended `return Result.Ok([])`, and
    /// every `?` in one produced an `Err` that meant nothing.
    #[test]
    fn a_test_that_answers_a_result_is_unwrapped_at_the_boundary() {
        let f = crate::testing::Fixture::build(&[(
            "lib.rs",
            "pub fn parse(s: &str) -> Result<usize, String> { Ok(s.len()) }\n\
             #[cfg(test)]\n\
             mod tests {\n\
               use super::*;\n\
               #[test]\n\
               fn answers_a_result() -> Result<(), String> {\n\
                 let n = parse(\"ab\")?;\n\
                 assert_eq!(n, 2);\n\
                 Ok(())\n\
               }\n\
               #[test]\n\
               fn answers_nothing() {\n\
                 assert_eq!(parse(\"ab\").unwrap(), 2);\n\
               }\n\
             }",
        )]);
        let ts = generate_test_ts_with_imports(&f.reg, &f.files[0].file, "lib.rs", &HashMap::new(), "index")
            .expect("the file declares tests");
        let answering = ts
            .split("test('answers_a_result'")
            .nth(1)
            .expect("the Result-answering test is emitted");
        let answering = answering.split("test('").next().unwrap();
        assert!(answering.contains("})().unwrap();"), "{}", answering);
        let plain = ts.split("test('answers_nothing'").nth(1).expect("the other test is emitted");
        assert!(!plain.contains("})().unwrap();"), "a test that answers nothing is left alone:\n{}", plain);
    }

    /// A crate is ONE TypeScript module, so several Rust modules reach the same
    /// specifier and two passes both write a line for it: connector-local's
    /// emitted file opened with two `import { .. } from '@ankurah/core'` lines
    /// sharing four names, which is eight `TS2300` duplicate identifiers and
    /// was every own-file error that package had.
    #[test]
    fn one_import_line_per_module() {
        let written = "// MIRRORS: x\n\
                       import { Node, PolicyAgent } from '@ankurah/core';\n\
                       import { Node, SendError } from '@ankurah/core';\n\
                       \n\
                       export class A {}\n";
        let out = merge_named_imports(written, &extra(&[("@ankurah/core", &["WeakNode"])]));
        assert_eq!(out.matches("from '@ankurah/core'").count(), 1, "{}", out);
        assert!(out.contains("import { Node, PolicyAgent, SendError, WeakNode } from '@ankurah/core';"), "{}", out);
        assert!(out.contains("export class A {}"), "{}", out);
    }

    /// The merged block stands where the first import stood, so the file still
    /// opens with its `// MIRRORS:` line and the code keeps its order.
    #[test]
    fn the_merged_block_stands_where_the_imports_did() {
        let written = "// MIRRORS: x\nimport { A } from './a';\nexport const b = 1;\n";
        let out = merge_named_imports(written, &extra(&[]));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "// MIRRORS: x");
        assert_eq!(lines[1], "import { A } from './a';");
        assert_eq!(lines[2], "export const b = 1;");
    }

    /// A namespace import and a side-effect import are not named imports and
    /// keep their own lines.
    #[test]
    fn only_named_imports_are_merged() {
        let written = "import * as proto from '@ankurah/proto';\nimport './side-effect';\nimport { A } from './a';\n";
        let out = merge_named_imports(written, &extra(&[]));
        assert!(out.contains("import * as proto from '@ankurah/proto';"), "{}", out);
        assert!(out.contains("import './side-effect';"), "{}", out);
        assert!(out.contains("import { A } from './a';"), "{}", out);
    }

    #[test]
    fn a_named_import_line_is_read_as_its_module_and_names() {
        let (module, names) = named_import("import { A, B as C } from './x';").unwrap();
        assert_eq!(module, "./x");
        assert_eq!(names, vec!["A".to_string(), "B as C".to_string()]);
        assert!(named_import("import * as m from './x';").is_none());
        assert!(named_import("export { A } from './x';").is_none());
    }
}
