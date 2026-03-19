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
}

#[derive(Debug)]
pub struct TraitInfo {
    pub name: String,
    pub is_pub: bool,
    pub methods: Vec<FnInfo>,
    pub has_default_impls: bool,
    pub generics: String,
}

#[derive(Debug)]
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
    /// Translated function body (None = stub, Some = translated)
    pub body_ts: Option<String>,
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
