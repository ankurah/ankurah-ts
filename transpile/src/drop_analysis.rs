//! Static analysis: which types transitively contain Drop types?
//!
//! Walk all .rs files in a directory, parse with syn, and determine:
//! 1. Which types have `impl Drop for T`
//! 2. Which types contain fields whose types are (transitively) Drop
//!
//! This answers the ownership question: only types that transitively
//! contain Drop types need `using` declarations in the TS port.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use syn::{self, visit::Visit, Type, Fields, GenericArgument, PathArguments};
use walkdir::WalkDir;

/// Information about a Rust type
#[derive(Debug)]
struct TypeInfo {
    /// Name of the type
    name: String,
    /// File where it's defined
    file: String,
    /// Names of types referenced in fields
    field_types: Vec<String>,
    /// Whether this type has `impl Drop`
    has_drop: bool,
    /// Whether this is an enum (vs struct)
    is_enum: bool,
}

/// Visitor that collects type definitions and Drop impls
struct TypeCollector {
    /// Map of type name -> TypeInfo
    types: HashMap<String, TypeInfo>,
    /// Current file being parsed
    current_file: String,
}

impl TypeCollector {
    fn new() -> Self {
        Self {
            types: HashMap::new(),
            current_file: String::new(),
        }
    }

    /// Extract type names from a field type (handles generics like Arc<T>, Vec<T>, Option<T>)
    fn extract_type_names(ty: &Type) -> Vec<String> {
        let mut names = Vec::new();
        Self::extract_type_names_inner(ty, &mut names);
        names
    }

    fn extract_type_names_inner(ty: &Type, names: &mut Vec<String>) {
        match ty {
            Type::Path(type_path) => {
                if let Some(segment) = type_path.path.segments.last() {
                    let name = segment.ident.to_string();
                    names.push(name);

                    // Recurse into generic arguments (Arc<T>, Vec<T>, Option<T>, etc.)
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let GenericArgument::Type(inner_ty) = arg {
                                Self::extract_type_names_inner(inner_ty, names);
                            }
                        }
                    }
                }
            }
            Type::Reference(type_ref) => {
                Self::extract_type_names_inner(&type_ref.elem, names);
            }
            Type::Tuple(type_tuple) => {
                for elem in &type_tuple.elems {
                    Self::extract_type_names_inner(elem, names);
                }
            }
            _ => {}
        }
    }

    /// Extract field type names from struct/enum fields
    fn collect_field_types(fields: &Fields) -> Vec<String> {
        let mut types = Vec::new();
        match fields {
            Fields::Named(named) => {
                for field in &named.named {
                    types.extend(Self::extract_type_names(&field.ty));
                }
            }
            Fields::Unnamed(unnamed) => {
                for field in &unnamed.unnamed {
                    types.extend(Self::extract_type_names(&field.ty));
                }
            }
            Fields::Unit => {}
        }
        types
    }
}

impl<'ast> Visit<'ast> for TypeCollector {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        let name = node.ident.to_string();
        let field_types = Self::collect_field_types(&node.fields);

        self.types.entry(name.clone()).or_insert_with(|| TypeInfo {
            name,
            file: self.current_file.clone(),
            field_types,
            has_drop: false,
            is_enum: false,
        });

        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        let name = node.ident.to_string();
        let mut field_types = Vec::new();

        for variant in &node.variants {
            field_types.extend(Self::collect_field_types(&variant.fields));
        }

        self.types.entry(name.clone()).or_insert_with(|| TypeInfo {
            name,
            file: self.current_file.clone(),
            field_types,
            has_drop: false,
            is_enum: true,
        });

        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        // Check for `impl Drop for T`
        if let Some((_, trait_path, _)) = &node.trait_ {
            if let Some(segment) = trait_path.segments.last() {
                if segment.ident == "Drop" {
                    // Extract the target type name
                    if let Type::Path(type_path) = &*node.self_ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            let name = segment.ident.to_string();
                            self.types
                                .entry(name.clone())
                                .and_modify(|info| info.has_drop = true)
                                .or_insert_with(|| TypeInfo {
                                    name,
                                    file: self.current_file.clone(),
                                    field_types: Vec::new(),
                                    has_drop: true,
                                    is_enum: false,
                                });
                        }
                    }
                }
            }
        }

        syn::visit::visit_item_impl(self, node);
    }
}

/// Compute transitive Drop closure: which types transitively contain Drop types?
fn compute_transitive_drop(types: &HashMap<String, TypeInfo>) -> HashSet<String> {
    let mut drop_types: HashSet<String> = HashSet::new();

    // Seed with direct Drop impls
    for (name, info) in types {
        if info.has_drop {
            drop_types.insert(name.clone());
        }
    }

    // Fixed-point iteration: if any field type is in drop_types, add the parent
    loop {
        let mut changed = false;
        for (name, info) in types {
            if drop_types.contains(name) {
                continue;
            }
            for field_type in &info.field_types {
                if drop_types.contains(field_type) {
                    drop_types.insert(name.clone());
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }

    drop_types
}

pub fn analyze(path: &Path) -> Result<()> {
    let mut collector = TypeCollector::new();

    // Walk all .rs files
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let file_path = entry.path();
        let relative = file_path.strip_prefix(path).unwrap_or(file_path);
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;

        let syntax = syn::parse_file(&content)
            .with_context(|| format!("Failed to parse {}", file_path.display()))?;

        collector.current_file = relative.display().to_string();
        collector.visit_file(&syntax);
    }

    // Compute transitive closure
    let drop_types = compute_transitive_drop(&collector.types);

    // Report
    println!("=== Types with direct `impl Drop` ===\n");
    let mut direct: Vec<_> = collector.types.values()
        .filter(|t| t.has_drop)
        .collect();
    direct.sort_by_key(|t| &t.name);
    for t in &direct {
        println!("  {} ({})", t.name, t.file);
    }

    println!("\n=== Types that transitively contain Drop types ===\n");
    let mut transitive: Vec<_> = collector.types.values()
        .filter(|t| drop_types.contains(&t.name) && !t.has_drop)
        .collect();
    transitive.sort_by_key(|t| &t.name);
    for t in &transitive {
        let drop_fields: Vec<_> = t.field_types.iter()
            .filter(|ft| drop_types.contains(*ft))
            .collect();
        println!("  {} ({}) — via {:?}", t.name, t.file, drop_fields);
    }

    println!("\n=== Value types (NO transitive Drop) ===\n");
    let mut value_types: Vec<_> = collector.types.values()
        .filter(|t| !drop_types.contains(&t.name))
        .collect();
    value_types.sort_by_key(|t| &t.name);
    for t in &value_types {
        let kind = if t.is_enum { "enum" } else { "struct" };
        println!("  {} [{}] ({})", t.name, kind, t.file);
    }

    println!("\n=== Summary ===");
    println!("  Total types: {}", collector.types.len());
    println!("  Direct Drop: {}", direct.len());
    println!("  Transitive Drop: {}", transitive.len());
    println!("  Value types: {}", value_types.len());

    Ok(())
}
