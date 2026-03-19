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

    // Collect all referenced types
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
        }
    }
    for f in &file.functions {
        imports::collect_type_refs(&f.return_type, &mut referenced);
        for p in &f.params { imports::collect_type_refs(&p.ty, &mut referenced); }
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
        import_lines.push_str(&format!("import {{ {} }} from '{}';\n", types.join(", "), module));
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

    // Collect local type names
    let mut local_types: HashSet<String> = HashSet::new();
    for s in &file.structs { local_types.insert(s.name.clone()); }
    for e in &file.enums { local_types.insert(e.name.clone()); }
    for t in &file.traits { local_types.insert(t.name.clone()); }

    // Base imports (@ankurah/base)
    let mut base_imports: Vec<&str> = Vec::new();
    if !file.structs.is_empty() { base_imports.push("Struct"); }
    if !file.enums.is_empty() { base_imports.push("Enum"); }
    for imp in &file.impls {
        if let Some(trait_name) = &imp.trait_name {
            if trait_name == "Drop" && !base_imports.contains(&"Drop") {
                base_imports.push("Drop");
            }
        }
    }
    if !base_imports.is_empty() {
        out.push_str(&format!("import {{ {} }} from '@ankurah/base';\n", base_imports.join(", ")));
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

    // Bincode imports
    let needs_bincode = file.structs.iter().any(|s| crate::bincode_module::has_serde_derive(&s.derives))
        || file.enums.iter().any(|e| crate::bincode_module::has_serde_derive(&e.derives));
    if needs_bincode {
        out.push_str("import { BincodeReader, BincodeWriter } from './codec';\n");
    }

    // Remaining unresolved external type references
    let mut referenced_types: HashSet<String> = HashSet::new();
    for s in &file.structs {
        for f in &s.fields { imports::collect_type_refs(&f.ty, &mut referenced_types); }
    }
    for e in &file.enums {
        for v in &e.variants {
            for f in &v.fields { imports::collect_type_refs(&f.ty, &mut referenced_types); }
        }
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

    out.push('\n');

    // Organize impl blocks
    let mut inherent_methods: HashMap<String, Vec<&FnInfo>> = HashMap::new();
    let mut trait_impls: HashMap<String, Vec<&str>> = HashMap::new();
    let mut trait_methods: HashMap<String, Vec<(&str, &[String], &FnInfo)>> = HashMap::new();

    for imp in &file.impls {
        if let Some(trait_name) = &imp.trait_name {
            trait_impls.entry(imp.target_type.clone()).or_default().push(trait_name.as_str());
            for method in &imp.methods {
                trait_methods.entry(imp.target_type.clone())
                    .or_default()
                    .push((trait_name.as_str(), &imp.trait_type_args, method));
            }
        } else {
            inherent_methods.entry(imp.target_type.clone()).or_default().extend(imp.methods.iter());
        }
    }

    // Build FQN prefix from crate_path: "proto/src/error.rs" → "ankurah_proto::error"
    let fqn_prefix = crate_path_to_fqn_prefix(rust_crate_path);

    // Emit items (skip fully provided types)
    for s in &file.structs {
        let fqn = format!("{}::{}", fqn_prefix, s.name);
        if config.map_or(false, |c| c.is_provided(&fqn)) {
            out.push_str(&format!("// PROVIDED: {} — hand-written implementation preserved\n\n", s.name));
            continue;
        }
        emit::emit_struct(&mut out, s, &inherent_methods, &trait_impls, &trait_methods);
    }
    for e in &file.enums {
        let fqn = format!("{}::{}", fqn_prefix, e.name);
        if config.map_or(false, |c| c.is_provided(&fqn)) {
            out.push_str(&format!("// PROVIDED: {} — hand-written implementation preserved\n\n", e.name));
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

    out
}

/// Generate test file content from extracted test functions
pub fn generate_test_ts(file: &RustFile, rust_crate_path: &str) -> Option<String> {
    if file.test_functions.is_empty() {
        return None;
    }

    let mut out = String::new();
    out.push_str(&format!("// MIRRORS: ankurah/{} (tests module)\n\n", rust_crate_path));
    out.push_str("import { describe, test, expect } from 'bun:test';\n\n");

    // Extract module name from crate path for describe block
    let module_name = rust_crate_path
        .rsplit('/')
        .next()
        .unwrap_or(rust_crate_path)
        .replace(".rs", "");

    out.push_str(&format!("describe('{} unit tests', () => {{\n", module_name));

    for f in &file.test_functions {
        let test_name = &f.name;
        let async_kw = if f.is_async { "async " } else { "" };
        out.push_str(&format!("  test('{}', {}() => {{\n    throw new Error('TODO');\n  }});\n\n",
            test_name, async_kw));
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
