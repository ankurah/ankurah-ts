//! Top-level TS code generation — orchestrates imports, emission, and output

use std::collections::{HashMap, HashSet};

use crate::types::*;
use crate::emit;
use crate::imports;

/// Generate TypeScript skeleton with resolved imports (used by batch command)
pub fn generate_ts_with_imports(
    file: &RustFile,
    rust_crate_path: &str,
    type_to_file: &HashMap<String, String>,
    current_module: &str,
) -> String {
    generate_ts_with_imports_configured(file, rust_crate_path, type_to_file, current_module, None)
}

pub fn generate_ts_with_imports_configured(
    file: &RustFile,
    rust_crate_path: &str,
    type_to_file: &HashMap<String, String>,
    current_module: &str,
    config: Option<&crate::config::Config>,
) -> String {
    let base = generate_ts_inner(file, rust_crate_path, config);

    let mut local_types: HashSet<String> = HashSet::new();
    for s in &file.structs { local_types.insert(s.name.clone()); }
    for e in &file.enums { local_types.insert(e.name.clone()); }
    for t in &file.traits { local_types.insert(t.name.clone()); }

    // Collect all referenced types — including from function/method bodies
    let mut referenced: HashSet<String> = HashSet::new();
    for s in &file.structs {
        for f in &s.fields { imports::collect_type_refs(&f.ty, &mut referenced); }
    }
    for e in &file.enums {
        for v in &e.variants {
            for f in &v.fields { imports::collect_type_refs(&f.ty, &mut referenced); }
        }
    }
    for imp in &file.impls {
        for m in &imp.methods {
            imports::collect_type_refs(&m.return_type, &mut referenced);
            for p in &m.params { imports::collect_type_refs(&p.ty, &mut referenced); }
            if let Some(b) = &m.body_ts { imports::collect_type_refs(b, &mut referenced); }
        }
    }
    for f in &file.functions {
        imports::collect_type_refs(&f.return_type, &mut referenced);
        for p in &f.params { imports::collect_type_refs(&p.ty, &mut referenced); }
        if let Some(b) = &f.body_ts { imports::collect_type_refs(b, &mut referenced); }
    }
    for decl in &file.module_decls {
        imports::collect_type_refs(decl, &mut referenced);
    }
    // Trait names from `implements` clauses
    for imp in &file.impls {
        if let Some(trait_name) = &imp.trait_name {
            referenced.insert(trait_name.clone());
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
pub fn generate_ts(file: &RustFile, rust_crate_path: &str) -> String {
    generate_ts_inner(file, rust_crate_path, None)
}

/// Generate with config awareness
pub fn generate_ts_configured(file: &RustFile, rust_crate_path: &str, config: &crate::config::Config) -> String {
    generate_ts_inner(file, rust_crate_path, Some(config))
}

fn generate_ts_inner(file: &RustFile, rust_crate_path: &str, config: Option<&crate::config::Config>) -> String {
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
        if let Some(trait_name) = &imp.trait_name {
            if trait_name == "Drop" && !base_imports.contains(&"Drop") {
                base_imports.push("Drop");
            }
        }
    }
    // Auto-detect base types used in fields, return types, and method bodies
    let mut all_type_refs = String::new();
    for s in &file.structs {
        if provided_set.contains(&s.name) { continue; }
        for f in &s.fields { all_type_refs.push_str(&f.ty); all_type_refs.push(' '); }
    }
    for e in &file.enums {
        if provided_set.contains(&e.name) { continue; }
        for v in &e.variants { for f in &v.fields { all_type_refs.push_str(&f.ty); all_type_refs.push(' '); } }
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
    let base_runtime_types = ["Result", "Arc", "Weak", "Mutex", "MutexGuard",
        "RwLock", "RwLockReadGuard", "RwLockWriteGuard",
        "RefCell", "Ref", "RefMut", "ThreadLocal"];
    for ty in &base_runtime_types {
        // Don't import if the file defines its own type with the same name
        if all_type_refs.contains(ty) && !base_imports.contains(ty) && !local_types.contains(*ty) {
            base_imports.push(ty);
        }
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
        for f in &s.fields { imports::collect_type_refs(&f.ty, &mut referenced_types); }
    }
    for e in &file.enums {
        if provided_set.contains(&e.name) { continue; }
        for v in &e.variants {
            for f in &v.fields { imports::collect_type_refs(&f.ty, &mut referenced_types); }
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

    out.push('\n');

    // Organize impl blocks
    let mut inherent_methods: HashMap<String, Vec<&FnInfo>> = HashMap::new();
    let mut trait_impls: HashMap<String, Vec<(&str, &[String])>> = HashMap::new();
    let mut trait_methods: HashMap<String, Vec<(&str, &[String], &FnInfo)>> = HashMap::new();

    for imp in &file.impls {
        if let Some(trait_name) = &imp.trait_name {
            trait_impls.entry(imp.target_type.clone()).or_default().push((trait_name.as_str(), &imp.trait_type_args));
            for method in &imp.methods {
                trait_methods.entry(imp.target_type.clone())
                    .or_default()
                    .push((trait_name.as_str(), &imp.trait_type_args, method));
            }
        } else {
            inherent_methods.entry(imp.target_type.clone()).or_default().extend(imp.methods.iter());
        }
    }

    // Collect generic bounds from all impl blocks for each type.
    // Merges inline bounds + where clause bounds across all impls.
    let mut impl_bounds: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    for imp in &file.impls {
        if !imp.generic_bounds.is_empty() {
            let type_bounds = impl_bounds.entry(imp.target_type.clone()).or_default();
            for (param, bounds) in &imp.generic_bounds {
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
        emit::emit_struct(&mut out, s, &inherent_methods, &trait_impls, &trait_methods, impl_bounds.get(&s.name));
    }
    for e in &file.enums {
        if provided_set.contains(&e.name) {
            continue;
        }
        emit::emit_enum(&mut out, e, &inherent_methods, &trait_impls, &trait_methods);
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
        let export = if c.is_pub { "export " } else { "" };
        out.push_str(&format!("{}const {}: {} = undefined as any; // TODO\n\n", export, c.name, c.ty));
    }

    // Module-level declarations (thread_local, etc.)
    for decl in &file.module_decls {
        out.push_str(decl);
        out.push_str("\n\n");
    }

    out
}

/// Generate test file content from extracted test functions
pub fn generate_test_ts(file: &RustFile, rust_crate_path: &str) -> Option<String> {
    generate_test_ts_with_imports(file, rust_crate_path, &HashMap::new(), ".")
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
    for f in &file.test_functions {
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
    let base_runtime_types = ["Result", "Arc", "Weak", "Mutex", "MutexGuard",
        "RwLock", "RwLockReadGuard", "RwLockWriteGuard",
        "RefCell", "Ref", "RefMut", "ThreadLocal", "Struct", "Enum", "Drop"];
    let base_imports: Vec<&&str> = base_runtime_types.iter()
        .filter(|t| test_refs.contains(**t) && !available_types.contains(**t))
        .collect();
    if !base_imports.is_empty() {
        let mut sorted = base_imports;
        sorted.sort();
        out.push_str(&format!("import {{ {} }} from '@ankurah/base';\n",
            sorted.iter().map(|s| **s).collect::<Vec<_>>().join(", ")));
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
    let all_test_body: String = file.test_functions.iter()
        .filter_map(|f| f.body_ts.as_deref())
        .collect::<Vec<_>>().join(" ");
    if all_test_body.contains("BincodeWriter") || all_test_body.contains("BincodeReader") {
        out.push_str("import { BincodeWriter, BincodeReader } from './codec';\n");
    }
    out.push('\n');

    out.push_str(&format!("describe('{} unit tests', () => {{\n", module_name));

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
