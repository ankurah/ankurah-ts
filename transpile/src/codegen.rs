//! Top-level TS code generation — orchestrates imports, emission, and output

use std::collections::{HashMap, HashSet};

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

    // Build import lines
    let mut import_lines = String::new();
    let mut sorted_modules: Vec<&String> = imports_by_module.keys().collect();
    sorted_modules.sort();
    for module in sorted_modules {
        let mut types = imports_by_module[module].clone();
        types.sort();
        let import_path = relative_import_path(current_module, module);
        import_lines.push_str(&format!("import {{ {} }} from '{}';\n", types.join(", "), import_path));
    }

    // Import functions from inline modules.
    // Scan bodies for function names that exist in inline modules.
    for (mod_name, sub_file) in &file.inline_modules {
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
            found.sort();
            let import_path = relative_import_path(current_module, &sub_module);
            import_lines.push_str(&format!("import {{ {} }} from '{}';\n",
                found.join(", "), import_path));
        }
    }

    // Replace the TODO imports line
    if import_lines.is_empty() {
        base.lines()
            .filter(|l| !l.starts_with("// TODO imports:"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    } else {
        let mut result = String::new();
        for line in base.lines() {
            if line.starts_with("// TODO imports:") {
                result.push_str(&import_lines);
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        result
    }
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
pub(crate) const BASE_RUNTIME_SYMBOLS: [&str; 47] = [
    "Result", "Arc", "Weak", "Mutex", "MutexGuard",
    "RwLock", "RwLockReadGuard", "RwLockWriteGuard",
    "RefCell", "Ref", "RefMut", "ThreadLocal",
    // The closure that owns its captures, and the error `?` converts into.
    "OwnedClosure", "AnyhowError", "anyhow",
    // What an emitted `fromJson` answers with: serde_json::Error's stand-in.
    "JsonError",
    // The logger every `tracing::` macro writes a call on.
    "tracing",
    // What a consuming match arm releases the payload it took no name for
    // with, and Rust's two eager boolean operators.
    "dropUnbound", "boolAnd", "boolOr",
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
    let emitted = generate_declarations(reg, file, &provided_set);

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
    for (name, vis) in &file.mod_decls {
        if *vis != crate::types::VisInfo::Public {
            continue;
        }
        let target = provided_child_module(corpus_path, name, config)
            .unwrap_or_else(|| child_module(&file.path, name));
        let line = format!("export * from '{}';\n", target);
        if !out.contains(&line) {
            out.push(line);
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
    for u in &file.uses {
        if u.vis != crate::types::VisInfo::Public {
            continue;
        }
        for binding in &u.bindings {
            let line = match (&binding.local, &binding.path[..]) {
                (None, [name]) if children.contains_key(name) => format!(
                    "export * from '{}';\n",
                    provided_child_module(corpus_path, name, config)
                        .unwrap_or_else(|| child_module(&file.path, name))
                ),
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
    out
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
    let free: Vec<crate::emit_impls::FreeFn> = match reg.modules().lookup_file(&file.path) {
        Some(module) => crate::emit_impls::free_functions(reg, module, file),
        None => Vec::new(),
    };
    // A trait this file declares carries the function that picks among its
    // impls at run time, for the calls that dispatch through a bound the engine
    // cannot close.
    let dispatchers: Vec<crate::emit_impls::Dispatcher> =
        match reg.modules().lookup_file(&file.path) {
            Some(module) => crate::emit_impls::dispatchers(reg, module, file),
            None => Vec::new(),
        };
    let on_a_class = |imp: &ImplInfo| match reg.modules().lookup_file(&file.path) {
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
        emit::emit_struct(&mut out, reg, s, &inherent_methods, &trait_impls, &trait_methods, impl_bounds.get(&s.name), &file.assigned_fields);
    }
    for e in &file.enums {
        if provided_set.contains(&e.name) {
            continue;
        }
        emit::emit_enum(&mut out, reg, e, &inherent_methods, &trait_impls, &trait_methods);
    }
    for t in &file.traits {
        emit::emit_trait(&mut out, t);
    }
    for f in &file.functions {
        if !f.is_test {
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
        out.push_str(&format!("{}const {}: {} = undefined as any; // TODO\n\n", export, c.name, c.ty));
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

    // Import types from the parent module (same file)
    let local_imports: Vec<&String> = test_refs.iter()
        .filter(|t| available_types.contains(*t))
        .collect();
    if !local_imports.is_empty() {
        let mut sorted = local_imports;
        sorted.sort();
        out.push_str(&format!("import {{ {} }} from './{}';\n",
            sorted.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "), module_name));
    }

    // Import base types (Arc, Mutex, RefCell, etc.)
    let base_runtime_types = BASE_RUNTIME_SYMBOLS;
    let all_bodies: String = file
        .test_functions
        .iter()
        .chain(&file.test_helpers)
        .filter_map(|f| f.body_ts.as_deref())
        .collect::<Vec<_>>()
        .join(" ");
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
    if !base_imports.is_empty() {
        let mut sorted = base_imports;
        sorted.sort();
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

    out.push_str(&format!("describe('{} unit tests', () => {{\n", module_name));

    // The helpers first: every test that calls one is written below it.
    for f in &file.test_helpers {
        let params: Vec<String> = f
            .params
            .iter()
            .filter(|p| !p.is_self)
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
