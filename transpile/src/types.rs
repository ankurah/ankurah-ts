//! Data structures representing extracted Rust items
//!
//! Extraction keeps the `syn::Type` of everything it reads, because that is
//! what the type engine resolves. The TypeScript strings alongside them are
//! emission's business and are produced by `name_map`, never parsed back.

use crate::ty::Ty;

/// Visibility as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisInfo {
    Public,
    Crate,
    Super,
    Private,
    /// `pub(in some::path)`, which the engine does not narrow further than
    /// `pub(crate)`. The corpus writes none; the position is kept so that the
    /// first one to appear is reported rather than quietly widened.
    InPath { line: usize, col: usize },
}

/// Extracted information about a Rust source file
#[derive(Debug)]
pub struct RustFile {
    pub path: String,
    /// The visibility of the `mod` declaration this file or inline module came
    /// from. A file module is public; an inline `mod x` may not be.
    pub vis: VisInfo,
    pub structs: Vec<StructInfo>,
    pub enums: Vec<EnumInfo>,
    pub traits: Vec<TraitInfo>,
    pub functions: Vec<FnInfo>,
    pub impls: Vec<ImplInfo>,
    pub uses: Vec<UseInfo>,
    pub type_aliases: Vec<TypeAliasInfo>,
    pub consts: Vec<ConstInfo>,
    /// Test functions from #[cfg(test)] mod tests { ... }
    pub test_functions: Vec<FnInfo>,
    /// Raw TS declarations to emit at module level (e.g., thread_local → const)
    pub module_decls: Vec<String>,
    /// Inline modules extracted as separate files.
    /// (module_name, RustFile) — emitted to parent_dir/module_name.ts
    pub inline_modules: Vec<(String, RustFile)>,
}

impl RustFile {
    pub fn empty(path: String) -> RustFile {
        RustFile {
            path,
            vis: VisInfo::Public,
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            functions: Vec::new(),
            impls: Vec::new(),
            uses: Vec::new(),
            type_aliases: Vec::new(),
            consts: Vec::new(),
            test_functions: Vec::new(),
            module_decls: Vec::new(),
            inline_modules: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct StructInfo {
    pub name: String,
    pub is_pub: bool,
    pub vis: VisInfo,
    pub fields: Vec<FieldInfo>,
    pub generics: String,
    /// Generic parameter names in declaration order, from syn.
    pub type_params: Vec<String>,
    pub derives: Vec<String>,
}

#[derive(Debug)]
pub struct FieldInfo {
    pub name: Option<String>,
    /// The Rust type as written — what the engine resolves, and what emission
    /// falls back to when it refused.
    pub rust_ty: syn::Type,
    /// The resolved type. `None` means the engine refused to name this type and
    /// filed a diagnostic; the fail-loud step turns that into an error.
    pub ty: Option<Ty>,
    pub is_pub: bool,
}

#[derive(Debug)]
pub struct EnumInfo {
    pub name: String,
    pub is_pub: bool,
    pub vis: VisInfo,
    pub variants: Vec<VariantInfo>,
    pub generics: String,
    pub type_params: Vec<String>,
    pub derives: Vec<String>,
}

#[derive(Debug)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    /// True if the variant has `#[serde(other)]` — catch-all for unknown discriminants
    pub is_serde_other: bool,
}

#[derive(Debug)]
pub struct TraitInfo {
    pub name: String,
    pub is_pub: bool,
    pub vis: VisInfo,
    pub methods: Vec<FnInfo>,
    pub has_default_impls: bool,
    pub generics: String,
    pub type_params: Vec<String>,
}

pub struct FnInfo {
    pub name: String,
    pub ts_name: String,
    pub is_pub: bool,
    pub vis: VisInfo,
    pub is_async: bool,
    pub is_static: bool,
    pub params: Vec<ParamInfo>,
    pub return_type: String,
    /// The written return type; `None` for a function that returns nothing.
    pub rust_return: Option<syn::Type>,
    pub generics: String,
    pub type_params: Vec<String>,
    pub is_test: bool,
    /// Raw syn AST for the function body — populated in Phase 1, consumed in Phase 3.
    /// Cloned from the parsed syn::File so it outlives the parse.
    pub body_ast: Option<syn::Block>,
    /// Translated function body (None = stub, Some = translated).
    /// Populated in Phase 3 (translate) from body_ast, or eagerly for legacy codepaths.
    pub body_ts: Option<String>,
}

impl std::fmt::Debug for FnInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnInfo")
            .field("name", &self.name)
            .field("ts_name", &self.ts_name)
            .field("is_pub", &self.is_pub)
            .field("is_static", &self.is_static)
            .field("return_type", &self.return_type)
            .field("body_ast", &self.body_ast.as_ref().map(|_| "<syn::Block>"))
            .field("body_ts", &self.body_ts)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub ty: String,
    /// The written type; `None` for the `self` receiver.
    pub rust_ty: Option<syn::Type>,
    pub is_self: bool,
    pub is_mut_self: bool,
}

#[derive(Debug)]
pub struct ImplInfo {
    pub target_type: String,
    /// The type the impl is written for, as written.
    pub self_ty: Option<syn::Type>,
    /// Generic parameter names declared by the impl block.
    pub type_params: Vec<String>,
    pub trait_name: Option<String>,
    /// For trait impls, the generic args (e.g., "String" for `impl From<String>`)
    pub trait_type_args: Vec<String>,
    pub methods: Vec<FnInfo>,
    /// Generic param bounds from the impl block (params + where clause).
    /// Maps param name → list of trait bound names.
    /// E.g., `impl<T: Clone + Send> Foo<T> where T: Debug` → {"T": ["Clone", "Debug"]}
    pub generic_bounds: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Debug)]
pub struct UseInfo {
    pub path: String,
    pub vis: VisInfo,
    /// What this `use` binds in its module, one entry per imported name.
    pub bindings: Vec<UseBindingInfo>,
}

/// One name a `use` brings into scope. `local` is `None` for `use path::*`.
#[derive(Debug, Clone)]
pub struct UseBindingInfo {
    pub local: Option<String>,
    pub path: Vec<String>,
}

#[derive(Debug)]
pub struct TypeAliasInfo {
    pub name: String,
    pub ty: String,
    pub rust_ty: syn::Type,
    pub is_pub: bool,
    pub vis: VisInfo,
    /// Generic parameter names the alias declares.
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConstInfo {
    pub name: String,
    pub ty: String,
    pub rust_ty: Option<syn::Type>,
    pub is_pub: bool,
    pub vis: VisInfo,
}

impl FieldInfo {
    /// The TypeScript type this field is emitted with.
    ///
    /// Produced from the resolved type. When the engine refused this type it
    /// filed a diagnostic, and emission keeps the syntactic mapping so that
    /// output stays comparable step to step; the fail-loud step removes the
    /// second arm along with every other fallback.
    pub fn ts_ty(&self, reg: &crate::registry::TypeRegistry) -> String {
        match &self.ty {
            Some(ty) => crate::name_map::map_ty(reg, ty),
            None => crate::name_map::map_type(&self.rust_ty),
        }
    }
}
