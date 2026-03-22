//! Type resolution layer — ResolvedType, TypeDef, TypeRegistry, ScopeStack
//!
//! This module provides type-aware context for body translation. Types are
//! represented structurally (ResolvedType) for resolution queries, while
//! string representations continue to be used for TS emission.
//!
//! System types (Arc, RwLock, etc.) and user-defined types are resolved
//! through the same mechanism — both are entries in the TypeRegistry.

use std::collections::HashMap;

// ── ResolvedType ──────────────────────────────────────────────────────

/// Structural type representation used for resolution during body translation.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    /// string, number, boolean, void, never, Uint8Array
    Primitive(String),

    /// User-defined, system, or external named type: EntityId, Arc<T>, Inner<T>
    /// System types (Arc, RwLock, etc.) are Named — their behavior comes from
    /// TypeDef metadata (deref_field, methods), not from the variant.
    Named { name: String, args: Vec<ResolvedType> },

    /// Generic type parameter (unresolved T, K, V)
    Param(String),

    /// T[] — Vec<T> maps here since TS syntax is T[] not Vec<T>
    Array(Box<ResolvedType>),

    /// [A, B] — tuple types
    Tuple(Vec<ResolvedType>),

    /// (params) => ret — function/closure types
    Fn { params: Vec<ResolvedType>, ret: Box<ResolvedType> },

    /// T | null — Option<T> maps here at resolution time
    Nullable(Box<ResolvedType>),

    /// Could not resolve
    Unknown,
}

impl ResolvedType {
    /// Replace type parameters according to a substitution map.
    /// Used when resolving method return types on generic types.
    ///
    /// Example: RwLock declares type_params=["T"] and method write returns
    /// RwLockWriteGuard<T>. Calling .write() on RwLock<Map<K,V>> substitutes
    /// T→Map<K,V>, returning RwLockWriteGuard<Map<K,V>>.
    pub fn substitute(&self, subst: &HashMap<&str, &ResolvedType>) -> ResolvedType {
        if subst.is_empty() {
            return self.clone();
        }
        match self {
            ResolvedType::Param(name) => {
                subst.get(name.as_str()).map(|t| (*t).clone()).unwrap_or_else(|| self.clone())
            }
            ResolvedType::Named { name, args } => ResolvedType::Named {
                name: name.clone(),
                args: args.iter().map(|a| a.substitute(subst)).collect(),
            },
            ResolvedType::Array(inner) =>
                ResolvedType::Array(Box::new(inner.substitute(subst))),
            ResolvedType::Nullable(inner) =>
                ResolvedType::Nullable(Box::new(inner.substitute(subst))),
            ResolvedType::Tuple(elems) =>
                ResolvedType::Tuple(elems.iter().map(|e| e.substitute(subst)).collect()),
            ResolvedType::Fn { params, ret } => ResolvedType::Fn {
                params: params.iter().map(|p| p.substitute(subst)).collect(),
                ret: Box::new(ret.substitute(subst)),
            },
            _ => self.clone(),
        }
    }

    /// Get the type name if this is a Named type
    pub fn name(&self) -> Option<&str> {
        match self {
            ResolvedType::Named { name, .. } => Some(name),
            _ => None,
        }
    }
}

// ── TypeDef ───────────────────────────────────────────────────────────

/// Definition of a type — user-defined structs/enums and provided system types
/// are both represented as TypeDef entries in the registry.
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub kind: TypeKind,
    /// Fields accessible on instances of this type
    pub fields: Vec<(String, ResolvedType)>,
    /// Methods with return types (for chained type inference).
    /// Keys are Rust names (snake_case); name_map handles TS conversion.
    pub methods: HashMap<String, MethodSig>,
    /// If accessing through this type requires an indirection in TS.
    ///   None         → not a deref type, look up fields directly
    ///   Some("")     → transparent deref (Box), unwrap to inner type, emit nothing
    ///   Some("value") → deref wrapper (Arc), emit .value, access inner type's fields
    pub deref_field: Option<String>,
    /// Generic type parameter names (e.g., ["T"] for Arc<T>, ["K", "V"] for HashMap<K,V>)
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct,
    Enum { variants: Vec<VariantDef> },
    Trait,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<(String, ResolvedType)>,
}

#[derive(Debug, Clone)]
pub struct MethodSig {
    pub params: Vec<(String, ResolvedType)>,
    pub ret: ResolvedType,
    pub is_static: bool,
}

// ── TypeRegistry ──────────────────────────────────────────────────────

/// Crate-wide type registry. Populated from parsed Rust sources + config-declared
/// provided types. Used for field lookups, method resolution, and enum detection.
#[derive(Debug)]
pub struct TypeRegistry {
    /// All known types: user-defined + provided + cross-crate.
    /// Keyed by Rust type name.
    types: HashMap<String, TypeDef>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        TypeRegistry { types: HashMap::new() }
    }

    /// Register a type definition
    pub fn register(&mut self, typedef: TypeDef) {
        self.types.insert(typedef.name.clone(), typedef);
    }

    /// Look up a type definition by name
    pub fn get(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
    }

    /// Is this name an enum?
    pub fn is_enum(&self, name: &str) -> bool {
        self.types.get(name).map_or(false, |td| matches!(td.kind, TypeKind::Enum { .. }))
    }

    /// Is this a valid variant of the given enum?
    pub fn is_variant(&self, type_name: &str, variant_name: &str) -> bool {
        if let Some(td) = self.types.get(type_name) {
            if let TypeKind::Enum { ref variants } = td.kind {
                return variants.iter().any(|v| v.name == variant_name);
            }
        }
        false
    }

    /// Resolve a field access on a typed expression.
    /// Returns (field_type, deref_accessor) where deref_accessor is:
    ///   None         → direct field access, no deref needed
    ///   Some("")     → transparent deref (Box), no accessor emitted
    ///   Some("value") → emit .value before field access
    ///
    /// Algorithm:
    ///   1. Look up type's TypeDef
    ///   2. If field exists directly → return (field_type, None)
    ///   3. If TypeDef has deref_field → unwrap inner type (with generic substitution),
    ///      recurse from step 1, return (field_type, Some(accessor))
    ///   4. If no deref_field → return None
    pub fn resolve_field(&self, ty: &ResolvedType, field: &str) -> Option<(ResolvedType, Option<String>)> {
        let (name, args) = match ty {
            ResolvedType::Named { name, args } => (name.as_str(), args.as_slice()),
            _ => return None,
        };

        let typedef = self.types.get(name)?;

        // Build substitution map for generic params
        let subst = self.build_subst(&typedef.type_params, args);

        // Check own fields first
        for (fname, ftype) in &typedef.fields {
            if fname == field {
                return Some((ftype.substitute(&subst), None));
            }
        }

        // Check deref — unwrap through wrapper types like Arc, Box
        // TODO: nested non-transparent deref chains (e.g., Arc<MutexGuard<Inner>>)
        // currently lose the inner deref accessor. In practice this is rare — you'd
        // call .lock() or .write() to get the guard, not nest it inside Arc. But if
        // it comes up, change return type to Vec<String> to accumulate deref chain.
        if let Some(ref accessor) = typedef.deref_field {
            if let Some(inner_ty) = args.first() {
                if let Some((resolved, inner_deref)) = self.resolve_field(inner_ty, field) {
                    let deref = if accessor.is_empty() {
                        // Transparent deref (Box) — pass through inner deref if any
                        inner_deref
                    } else {
                        // Non-transparent deref (Arc) — emit accessor, drop inner
                        // (see TODO above for nested non-transparent case)
                        Some(accessor.clone())
                    };
                    return Some((resolved, deref));
                }
            }
        }

        None
    }

    /// Resolve a method call on a typed expression.
    /// Returns the method's return type with generic params substituted.
    ///
    /// Algorithm:
    ///   1. Look up type's TypeDef
    ///   2. If method exists on TypeDef → return return_type (with generic substitution)
    ///   3. If TypeDef has deref_field → unwrap inner type, recurse
    ///   4. return None
    pub fn resolve_method(&self, ty: &ResolvedType, method: &str) -> Option<ResolvedType> {
        let (name, args) = match ty {
            // Handle Nullable (Option) methods as special cases
            ResolvedType::Nullable(inner) => {
                return self.resolve_nullable_method(inner, method);
            }
            ResolvedType::Named { name, args } => (name.as_str(), args.as_slice()),
            _ => return None,
        };

        let typedef = self.types.get(name)?;
        let subst = self.build_subst(&typedef.type_params, args);

        // Check own methods first
        if let Some(method_sig) = typedef.methods.get(method) {
            return Some(method_sig.ret.substitute(&subst));
        }

        // Check deref — recurse into inner type
        if typedef.deref_field.is_some() {
            if let Some(inner_ty) = args.first() {
                return self.resolve_method(inner_ty, method);
            }
        }

        None
    }

    /// Check if a method is defined directly on a type (not through deref).
    /// Used for deref suppression: arc.clone() should NOT deref.
    pub fn is_own_method(&self, ty: &ResolvedType, method: &str) -> bool {
        let name = match ty {
            ResolvedType::Named { name, .. } => name.as_str(),
            _ => return false,
        };
        self.types.get(name)
            .map_or(false, |td| td.methods.contains_key(method))
    }

    /// Check if a type has a deref_field (is a wrapper type)
    pub fn deref_field(&self, ty: &ResolvedType) -> Option<&str> {
        let name = match ty {
            ResolvedType::Named { name, .. } => name.as_str(),
            _ => return None,
        };
        self.types.get(name)
            .and_then(|td| td.deref_field.as_deref())
    }

    /// Resolve Option/Nullable methods as special cases
    fn resolve_nullable_method(&self, inner: &ResolvedType, method: &str) -> Option<ResolvedType> {
        match method {
            "unwrap" | "expect" => Some(inner.clone()),
            "is_some" | "is_none" => Some(ResolvedType::Primitive("boolean".to_string())),
            "map" => Some(ResolvedType::Unknown), // would need closure return type
            _ => None,
        }
    }

    /// Build a generic substitution map from type_params and actual args
    fn build_subst<'a>(&self, type_params: &'a [String], args: &'a [ResolvedType]) -> HashMap<&'a str, &'a ResolvedType> {
        type_params.iter()
            .zip(args.iter())
            .map(|(param, arg)| (param.as_str(), arg))
            .collect()
    }
}

// ── ScopeStack ────────────────────────────────────────────────────────

/// Stack of scopes for variable binding resolution.
/// Scopes are pushed on entry to impl/fn/block and popped on exit.
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

#[derive(Debug)]
pub struct Scope {
    pub kind: ScopeKind,
    pub bindings: HashMap<String, ResolvedType>,
}

#[derive(Debug)]
pub enum ScopeKind {
    /// Crate-level: all types visible
    Crate,
    /// Per-file: use imports resolved
    Module { use_imports: HashMap<String, String> },
    /// Per impl block: self_type bound
    Impl { self_type: ResolvedType },
    /// Per function: params bound
    Fn,
    /// Per { } block: let-bindings
    Block,
    /// Closure: captures from enclosing scope
    Closure,
}

impl ScopeStack {
    pub fn new() -> Self {
        ScopeStack { scopes: Vec::new() }
    }

    /// Push a new scope
    pub fn push(&mut self, scope: Scope) {
        self.scopes.push(scope);
    }

    /// Pop the innermost scope
    pub fn pop(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    /// Push a simple scope with no initial bindings
    pub fn push_block(&mut self) {
        self.scopes.push(Scope {
            kind: ScopeKind::Block,
            bindings: HashMap::new(),
        });
    }

    /// Push an Impl scope with self_type and field bindings on `this`
    pub fn push_impl(&mut self, self_type: ResolvedType) {
        let mut bindings = HashMap::new();
        bindings.insert("this".to_string(), self_type.clone());
        self.scopes.push(Scope {
            kind: ScopeKind::Impl { self_type },
            bindings,
        });
    }

    /// Push a Fn scope with param bindings
    pub fn push_fn(&mut self, params: Vec<(String, ResolvedType)>) {
        let bindings = params.into_iter().collect();
        self.scopes.push(Scope {
            kind: ScopeKind::Fn,
            bindings,
        });
    }

    /// Resolve a variable name, walking from innermost to outermost scope.
    /// Returns the first match (innermost scope wins — handles shadowing).
    pub fn resolve(&self, name: &str) -> Option<&ResolvedType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.bindings.get(name) {
                return Some(ty);
            }
        }
        None
    }

    /// Find the nearest Impl scope's self_type
    pub fn self_type(&self) -> Option<&ResolvedType> {
        for scope in self.scopes.iter().rev() {
            if let ScopeKind::Impl { ref self_type } = scope.kind {
                return Some(self_type);
            }
        }
        None
    }

    /// Bind a variable in the current (innermost) scope.
    /// If the name already exists in this scope, it is overwritten (Rust shadowing).
    pub fn bind(&mut self, name: String, ty: ResolvedType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name, ty);
        }
    }

    /// Resolve a type name through use-imports (for aliased type lookups).
    /// Checks Module scope use_imports, returns canonical name.
    pub fn resolve_type_name<'a>(&'a self, name: &'a str) -> &'a str {
        for scope in self.scopes.iter().rev() {
            if let ScopeKind::Module { ref use_imports } = scope.kind {
                if let Some(canonical) = use_imports.get(name) {
                    // Return the last segment of the canonical path
                    return canonical.rsplit("::").next().unwrap_or(canonical);
                }
            }
        }
        name
    }
}

// ── parse_type_string ─────────────────────────────────────────────────

/// Parse a TS-mapped type string into a ResolvedType.
/// Handles: Name<A, B>, T | null, T[], bare names.
///
/// IMPORTANT: This parses TS type syntax (post `name_map::map_type`), not Rust
/// type syntax. So it receives "Map<string, number>" not "HashMap<String, u32>",
/// and "T[]" not "Vec<T>". Both struct field types and method return types pass
/// through map_type before reaching this function.
///
/// This is syntactic parsing only — it builds a tree from angle-bracket syntax
/// and does not require the TypeRegistry to exist.
pub fn parse_type_string(s: &str) -> ResolvedType {
    let s = s.trim();
    if s.is_empty() || s == "()" {
        return ResolvedType::Primitive("void".to_string());
    }

    // Handle nullable: "T | null"
    if s.ends_with("| null") {
        let inner = s[..s.len() - 6].trim();
        return ResolvedType::Nullable(Box::new(parse_type_string(inner)));
    }

    // Handle array: "T[]"
    if s.ends_with("[]") {
        let inner = &s[..s.len() - 2];
        return ResolvedType::Array(Box::new(parse_type_string(inner)));
    }

    // Handle generic: "Name<A, B>"
    if let Some(bracket) = s.find('<') {
        let name = s[..bracket].trim();
        let inner = &s[bracket + 1..s.len() - 1]; // strip < >
        let args: Vec<ResolvedType> = split_type_args(inner)
            .iter()
            .map(|a| parse_type_string(a))
            .collect();
        return make_named_type(name, args);
    }

    // Bare name
    match s {
        "string" | "number" | "boolean" | "void" | "never" | "Uint8Array" | "bigint" =>
            ResolvedType::Primitive(s.to_string()),
        // Single uppercase letter → type parameter
        s if s.len() == 1 && s.chars().next().unwrap().is_uppercase() =>
            ResolvedType::Param(s.to_string()),
        _ => ResolvedType::Named { name: s.to_string(), args: vec![] },
    }
}

/// Split comma-separated type arguments, respecting nested brackets.
/// Tracks both `<>` and `[]` depth so that `Array<[K, V]>` doesn't split on the inner comma.
fn split_type_args(s: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' | '[' => depth += 1,
            '>' | ']' => depth -= 1,
            ',' if depth == 0 => {
                args.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = s[start..].trim();
    if !last.is_empty() {
        args.push(last);
    }
    args
}

/// Create a Named type, with special handling for Option and Tuple.
fn make_named_type(name: &str, args: Vec<ResolvedType>) -> ResolvedType {
    match name {
        // Option<T> → Nullable(T) — matches existing TS mapping where Option<T> is T | null
        "Option" if args.len() == 1 => ResolvedType::Nullable(Box::new(args.into_iter().next().unwrap())),
        // Tuple types: Tuple<A, B> → [A, B]
        "Tuple" => ResolvedType::Tuple(args),
        // Box<T> stays as Named("Box", [T]) — resolved via deref_field="" (transparent) in TypeRegistry.
        // All other types including system types (Arc, RwLock, etc.) are Named.
        _ => ResolvedType::Named { name: name.to_string(), args },
    }
}

// ── Build registry from extracted types ───────────────────────────────

use crate::types::*;

/// Build a TypeRegistry from extracted RustFile data and config-declared provided types.
pub fn build_registry(
    files: &[(String, RustFile)],
    provided_types: &[TypeDef],
) -> TypeRegistry {
    let mut registry = TypeRegistry::new();

    // Register provided types first (system types like Arc, RwLock, etc.)
    for td in provided_types {
        registry.register(td.clone());
    }

    // Register TS-mapped aliases for types where Rust name ≠ TS name.
    // Field types in StructInfo/FnInfo are already TS-mapped (via name_map::map_type),
    // so registry lookups use TS names. System types are declared under Rust names,
    // so we add aliases to ensure both paths resolve.
    let ts_aliases: &[(&str, &str)] = &[
        ("HashMap", "Map"),
        ("BTreeMap", "Map"),
        ("HashSet", "Set"),
        ("BTreeSet", "Set"),
    ];
    for &(rust_name, ts_name) in ts_aliases {
        if let Some(td) = registry.types.get(rust_name) {
            let mut alias = td.clone();
            alias.name = ts_name.to_string();
            registry.types.insert(ts_name.to_string(), alias);
        }
    }

    // Register types from parsed Rust files
    for (_path, file) in files {
        for s in &file.structs {
            registry.register(struct_to_typedef(s));
        }
        for e in &file.enums {
            registry.register(enum_to_typedef(e));
        }
        // Register method signatures from impl blocks
        for imp in &file.impls {
            if let Some(td) = registry.types.get_mut(&imp.target_type) {
                for method in &imp.methods {
                    td.methods.insert(method.name.clone(), MethodSig {
                        params: method.params.iter()
                            .map(|p| (p.name.clone(), parse_type_string(&p.ty)))
                            .collect(),
                        ret: parse_type_string(&method.return_type),
                        is_static: method.is_static,
                    });
                }
            }
        }
    }

    registry
}

/// Convert a StructInfo into a TypeDef
fn struct_to_typedef(s: &StructInfo) -> TypeDef {
    TypeDef {
        name: s.name.clone(),
        kind: TypeKind::Struct,
        fields: s.fields.iter().map(|f| {
            let name = f.name.clone().unwrap_or_else(|| "_0".to_string());
            (name, parse_type_string(&f.ty))
        }).collect(),
        methods: HashMap::new(),
        deref_field: None,
        type_params: extract_type_param_names(&s.generics),
    }
}

/// Convert an EnumInfo into a TypeDef
fn enum_to_typedef(e: &EnumInfo) -> TypeDef {
    let variants = e.variants.iter().map(|v| {
        VariantDef {
            name: v.name.clone(),
            fields: v.fields.iter().map(|f| {
                let name = f.name.clone().unwrap_or_else(|| "_0".to_string());
                (name, parse_type_string(&f.ty))
            }).collect(),
        }
    }).collect();

    TypeDef {
        name: e.name.clone(),
        kind: TypeKind::Enum { variants },
        fields: vec![],
        methods: HashMap::new(),
        deref_field: None,
        type_params: extract_type_param_names(&e.generics),
    }
}

/// Extract type parameter names from a generics string like "<T>" or "<K, V>"
fn extract_type_param_names(generics: &str) -> Vec<String> {
    if generics.is_empty() {
        return vec![];
    }
    let inner = generics.trim_start_matches('<').trim_end_matches('>');
    inner.split(',')
        .map(|s| {
            // Handle "T extends Foo" → just "T", "N: number" → just "N"
            let s = s.trim();
            s.split_whitespace().next().unwrap_or(s)
                .trim_end_matches(':').to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_type_string_primitive() {
        assert_eq!(parse_type_string("string"), ResolvedType::Primitive("string".to_string()));
        assert_eq!(parse_type_string("number"), ResolvedType::Primitive("number".to_string()));
        assert_eq!(parse_type_string("void"), ResolvedType::Primitive("void".to_string()));
    }

    #[test]
    fn test_parse_type_string_param() {
        assert_eq!(parse_type_string("T"), ResolvedType::Param("T".to_string()));
        assert_eq!(parse_type_string("K"), ResolvedType::Param("K".to_string()));
    }

    #[test]
    fn test_parse_type_string_named() {
        assert_eq!(parse_type_string("EntityId"), ResolvedType::Named {
            name: "EntityId".to_string(), args: vec![],
        });
    }

    #[test]
    fn test_parse_type_string_generic() {
        assert_eq!(parse_type_string("Arc<T>"), ResolvedType::Named {
            name: "Arc".to_string(),
            args: vec![ResolvedType::Param("T".to_string())],
        });
    }

    #[test]
    fn test_parse_type_string_nullable() {
        assert_eq!(parse_type_string("T | null"), ResolvedType::Nullable(
            Box::new(ResolvedType::Param("T".to_string()))
        ));
    }

    #[test]
    fn test_parse_type_string_array() {
        assert_eq!(parse_type_string("T[]"), ResolvedType::Array(
            Box::new(ResolvedType::Param("T".to_string()))
        ));
    }

    #[test]
    fn test_parse_type_string_option_becomes_nullable() {
        assert_eq!(parse_type_string("Option<T>"), ResolvedType::Nullable(
            Box::new(ResolvedType::Param("T".to_string()))
        ));
    }

    #[test]
    fn test_parse_type_string_box_stays_named() {
        // Box<T> stays as Named — transparent deref handled by TypeRegistry, not parse_type_string
        assert_eq!(parse_type_string("Box<T>"), ResolvedType::Named {
            name: "Box".to_string(),
            args: vec![ResolvedType::Param("T".to_string())],
        });
    }

    #[test]
    fn test_parse_type_string_nested() {
        let result = parse_type_string("RwLockWriteGuard<Map<K, V>>");
        assert_eq!(result, ResolvedType::Named {
            name: "RwLockWriteGuard".to_string(),
            args: vec![ResolvedType::Named {
                name: "Map".to_string(),
                args: vec![
                    ResolvedType::Param("K".to_string()),
                    ResolvedType::Param("V".to_string()),
                ],
            }],
        });
    }

    #[test]
    fn test_substitute() {
        let ty = parse_type_string("RwLockWriteGuard<T>");
        let map_type = ResolvedType::Named {
            name: "Map".to_string(),
            args: vec![
                ResolvedType::Primitive("number".to_string()),
                ResolvedType::Primitive("string".to_string()),
            ],
        };
        let mut subst = HashMap::new();
        subst.insert("T", &map_type);

        let result = ty.substitute(&subst);
        assert_eq!(result, ResolvedType::Named {
            name: "RwLockWriteGuard".to_string(),
            args: vec![ResolvedType::Named {
                name: "Map".to_string(),
                args: vec![
                    ResolvedType::Primitive("number".to_string()),
                    ResolvedType::Primitive("string".to_string()),
                ],
            }],
        });
    }

    #[test]
    fn test_registry_resolve_field_direct() {
        let mut registry = TypeRegistry::new();
        registry.register(TypeDef {
            name: "Foo".to_string(),
            kind: TypeKind::Struct,
            fields: vec![("bar".to_string(), ResolvedType::Primitive("number".to_string()))],
            methods: HashMap::new(),
            deref_field: None,
            type_params: vec![],
        });

        let ty = ResolvedType::Named { name: "Foo".to_string(), args: vec![] };
        let result = registry.resolve_field(&ty, "bar");
        assert!(result.is_some());
        let (field_ty, deref) = result.unwrap();
        assert_eq!(field_ty, ResolvedType::Primitive("number".to_string()));
        assert_eq!(deref, None);
    }

    #[test]
    fn test_registry_resolve_field_through_arc() {
        let mut registry = TypeRegistry::new();
        registry.register(TypeDef {
            name: "Arc".to_string(),
            kind: TypeKind::Struct,
            fields: vec![],
            methods: HashMap::new(),
            deref_field: Some("value".to_string()),
            type_params: vec!["T".to_string()],
        });
        registry.register(TypeDef {
            name: "Inner".to_string(),
            kind: TypeKind::Struct,
            fields: vec![("count".to_string(), ResolvedType::Primitive("number".to_string()))],
            methods: HashMap::new(),
            deref_field: None,
            type_params: vec![],
        });

        let ty = ResolvedType::Named {
            name: "Arc".to_string(),
            args: vec![ResolvedType::Named { name: "Inner".to_string(), args: vec![] }],
        };
        let result = registry.resolve_field(&ty, "count");
        assert!(result.is_some());
        let (field_ty, deref) = result.unwrap();
        assert_eq!(field_ty, ResolvedType::Primitive("number".to_string()));
        assert_eq!(deref, Some("value".to_string()));
    }

    #[test]
    fn test_registry_resolve_method_with_substitution() {
        let mut registry = TypeRegistry::new();
        let mut methods = HashMap::new();
        methods.insert("write".to_string(), MethodSig {
            params: vec![],
            ret: parse_type_string("RwLockWriteGuard<T>"),
            is_static: false,
        });
        registry.register(TypeDef {
            name: "RwLock".to_string(),
            kind: TypeKind::Struct,
            fields: vec![],
            methods,
            deref_field: None,
            type_params: vec!["T".to_string()],
        });

        let ty = ResolvedType::Named {
            name: "RwLock".to_string(),
            args: vec![ResolvedType::Named {
                name: "Map".to_string(),
                args: vec![
                    ResolvedType::Primitive("number".to_string()),
                    ResolvedType::Primitive("string".to_string()),
                ],
            }],
        };

        let result = registry.resolve_method(&ty, "write");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), ResolvedType::Named {
            name: "RwLockWriteGuard".to_string(),
            args: vec![ResolvedType::Named {
                name: "Map".to_string(),
                args: vec![
                    ResolvedType::Primitive("number".to_string()),
                    ResolvedType::Primitive("string".to_string()),
                ],
            }],
        });
    }

    #[test]
    fn test_registry_enum_detection() {
        let mut registry = TypeRegistry::new();
        registry.register(TypeDef {
            name: "Signal".to_string(),
            kind: TypeKind::Enum {
                variants: vec![
                    VariantDef { name: "Constant".to_string(), fields: vec![] },
                    VariantDef { name: "Dynamic".to_string(), fields: vec![] },
                ],
            },
            fields: vec![],
            methods: HashMap::new(),
            deref_field: None,
            type_params: vec![],
        });

        assert!(registry.is_enum("Signal"));
        assert!(registry.is_variant("Signal", "Constant"));
        assert!(!registry.is_variant("Signal", "Missing"));
        assert!(!registry.is_enum("Unknown"));
    }

    #[test]
    fn test_scope_stack_shadowing() {
        let mut stack = ScopeStack::new();
        stack.push(Scope {
            kind: ScopeKind::Fn,
            bindings: {
                let mut m = HashMap::new();
                m.insert("x".to_string(), ResolvedType::Primitive("number".to_string()));
                m
            },
        });
        stack.push_block();
        stack.bind("x".to_string(), ResolvedType::Primitive("string".to_string()));

        // Inner scope shadows outer
        assert_eq!(stack.resolve("x"), Some(&ResolvedType::Primitive("string".to_string())));

        stack.pop();
        // After popping, outer scope's x is visible again
        assert_eq!(stack.resolve("x"), Some(&ResolvedType::Primitive("number".to_string())));
    }

    #[test]
    fn test_nullable_method_resolution() {
        let registry = TypeRegistry::new();
        let ty = ResolvedType::Nullable(Box::new(ResolvedType::Primitive("string".to_string())));

        let result = registry.resolve_method(&ty, "unwrap");
        assert_eq!(result, Some(ResolvedType::Primitive("string".to_string())));

        let result = registry.resolve_method(&ty, "is_some");
        assert_eq!(result, Some(ResolvedType::Primitive("boolean".to_string())));
    }

    #[test]
    fn test_registry_resolve_field_through_box_transparent() {
        let mut registry = TypeRegistry::new();
        registry.register(TypeDef {
            name: "Box".to_string(),
            kind: TypeKind::Struct,
            fields: vec![],
            methods: HashMap::new(),
            deref_field: Some("".to_string()), // transparent
            type_params: vec!["T".to_string()],
        });
        registry.register(TypeDef {
            name: "Inner".to_string(),
            kind: TypeKind::Struct,
            fields: vec![("count".to_string(), ResolvedType::Primitive("number".to_string()))],
            methods: HashMap::new(),
            deref_field: None,
            type_params: vec![],
        });

        let ty = ResolvedType::Named {
            name: "Box".to_string(),
            args: vec![ResolvedType::Named { name: "Inner".to_string(), args: vec![] }],
        };
        let result = registry.resolve_field(&ty, "count");
        assert!(result.is_some());
        let (field_ty, deref) = result.unwrap();
        assert_eq!(field_ty, ResolvedType::Primitive("number".to_string()));
        assert_eq!(deref, None); // Box is transparent — no accessor emitted
    }

    #[test]
    fn test_parse_type_string_array_with_brackets() {
        // Array<[K, V]> — split_type_args tracks [] depth, so inner comma is preserved
        let result = parse_type_string("Array<[K, V]>");
        // [K, V] is parsed as a single arg string "[K, V]" which becomes Named("[K, V]", [])
        // This is a named type with brackets in the name — not ideal but doesn't break.
        // For cleaner semantics, use Tuple<K, V>[] in config strings.
        assert_eq!(result, ResolvedType::Named {
            name: "Array".to_string(),
            args: vec![ResolvedType::Named {
                name: "[K, V]".to_string(),
                args: vec![],
            }],
        });

        // Preferred config format: Tuple<K, V>[]
        let result = parse_type_string("Tuple<K, V>[]");
        assert_eq!(result, ResolvedType::Array(Box::new(
            ResolvedType::Tuple(vec![
                ResolvedType::Param("K".to_string()),
                ResolvedType::Param("V".to_string()),
            ])
        )));
    }

    #[test]
    fn test_registry_ts_aliases() {
        let system_types = vec![TypeDef {
            name: "HashMap".to_string(),
            kind: TypeKind::Struct,
            fields: vec![],
            methods: {
                let mut m = HashMap::new();
                m.insert("get".to_string(), MethodSig {
                    params: vec![],
                    ret: ResolvedType::Nullable(Box::new(ResolvedType::Param("V".to_string()))),
                    is_static: false,
                });
                m
            },
            deref_field: None,
            type_params: vec!["K".to_string(), "V".to_string()],
        }];
        let registry = build_registry(&[], &system_types);

        // Should be accessible under both "HashMap" and "Map"
        assert!(registry.get("HashMap").is_some());
        assert!(registry.get("Map").is_some());

        // Method resolution should work via TS name
        let map_ty = ResolvedType::Named {
            name: "Map".to_string(),
            args: vec![
                ResolvedType::Primitive("string".to_string()),
                ResolvedType::Primitive("number".to_string()),
            ],
        };
        let result = registry.resolve_method(&map_ty, "get");
        assert!(result.is_some());
        // V is substituted with number
        assert_eq!(result.unwrap(), ResolvedType::Nullable(
            Box::new(ResolvedType::Primitive("number".to_string()))
        ));
    }
}
