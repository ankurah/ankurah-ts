//! Data structures representing extracted Rust items

/// Extracted information about a Rust source file
#[derive(Debug)]
pub struct RustFile {
    pub path: String,
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
}

#[derive(Debug)]
pub struct StructInfo {
    pub name: String,
    pub is_pub: bool,
    pub fields: Vec<FieldInfo>,
    pub generics: String,
    pub derives: Vec<String>,
}

#[derive(Debug)]
pub struct FieldInfo {
    pub name: Option<String>,
    pub ty: String,
    pub rust_ty: String,
    pub is_pub: bool,
}

#[derive(Debug)]
pub struct EnumInfo {
    pub name: String,
    pub is_pub: bool,
    pub variants: Vec<VariantInfo>,
    pub generics: String,
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
    pub methods: Vec<FnInfo>,
    pub has_default_impls: bool,
    pub generics: String,
}

pub struct FnInfo {
    pub name: String,
    pub ts_name: String,
    pub is_pub: bool,
    pub is_async: bool,
    pub is_static: bool,
    pub params: Vec<ParamInfo>,
    pub return_type: String,
    pub generics: String,
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
    pub is_self: bool,
    pub is_mut_self: bool,
}

#[derive(Debug)]
pub struct ImplInfo {
    pub target_type: String,
    pub trait_name: Option<String>,
    /// For trait impls, the generic args (e.g., "String" for `impl From<String>`)
    pub trait_type_args: Vec<String>,
    pub methods: Vec<FnInfo>,
}

#[derive(Debug)]
pub struct UseInfo {
    pub path: String,
    pub is_pub: bool,
}

#[derive(Debug)]
pub struct TypeAliasInfo {
    pub name: String,
    pub ty: String,
    pub is_pub: bool,
}

#[derive(Debug)]
pub struct ConstInfo {
    pub name: String,
    pub ty: String,
    pub is_pub: bool,
}
