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
    /// `trait Signal: Debug` — the traits an implementor must also implement,
    /// as written. A method reached on a `dyn Signal` may be declared on one of
    /// these rather than on `Signal` itself.
    pub supertraits: Vec<syn::TraitBound>,
    /// `type Item;` — the associated types each impl has to supply.
    pub assoc_types: Vec<String>,
}

/// How a method takes its receiver. Method resolution picks the borrow it needs
/// from this: `fn len(&self)` is found on `&C` and never on `C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfKind {
    /// `self` or `mut self` — the receiver is taken by value.
    Value,
    /// `&self`.
    Ref,
    /// `&mut self`.
    RefMut,
    /// `self: Arc<Self>`, `self: Pin<&mut Self>` — a receiver written as a type.
    /// Reading one as by-value put the method on the wrong step of the deref
    /// chain, so it is reported and left out of the table until a step supports
    /// it; the written type travels on `FnInfo::self_receiver`.
    Arbitrary,
}

pub struct FnInfo {
    pub name: String,
    pub ts_name: String,
    pub is_pub: bool,
    pub vis: VisInfo,
    pub is_async: bool,
    pub is_static: bool,
    /// How the receiver is taken; `None` for an associated function with none.
    pub self_kind: Option<SelfKind>,
    /// The type an arbitrary receiver was written as, e.g. `Arc<Self>`.
    pub self_receiver: Option<syn::Type>,
    /// True for a trait method the trait wrote a body for, so an impl need not.
    /// The body is extracted with it and emitted on the abstract class.
    pub has_default_body: bool,
    pub params: Vec<ParamInfo>,
    pub return_type: String,
    /// The written return type; `None` for a function that returns nothing.
    pub rust_return: Option<syn::Type>,
    pub generics: String,
    pub type_params: Vec<String>,
    /// The function's generics as written, so the bounds on its parameters are
    /// available where a call on one of them has to be resolved.
    pub syn_generics: syn::Generics,
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

/// An `impl` block as written.
///
/// The trait it implements is kept as the `syn::Path` the source wrote and the
/// generics as `syn::Generics`, so that the engine resolves both against the
/// registry. The TypeScript spellings emission needs are derived from those
/// below; nothing goes out as a string and comes back as a type.
#[derive(Debug)]
pub struct ImplInfo {
    /// The class the emitted methods are written onto. Emission's business:
    /// the engine reads `self_ty`.
    pub target_type: String,
    /// The type the impl is written for, as written.
    pub self_ty: Option<syn::Type>,
    /// Generic parameter names declared by the impl block.
    pub type_params: Vec<String>,
    /// `impl Deref for X` — the trait's path as written, with its arguments.
    pub trait_path: Option<syn::Path>,
    /// The impl block's generics, carrying both the inline bounds and the
    /// `where` clause.
    pub generics: syn::Generics,
    /// `type Target = T;` — what this impl supplies for the trait's associated
    /// types.
    pub assoc_types: Vec<(String, syn::Type)>,
    pub methods: Vec<FnInfo>,
}

impl ImplInfo {
    /// The trait's name as TypeScript writes it: the path's last segment.
    pub fn trait_name(&self) -> Option<String> {
        let path = self.trait_path.as_ref()?;
        Some(path.segments.last()?.ident.to_string())
    }

    /// The trait's type arguments, in the TypeScript spelling emission puts in
    /// an `implements` clause and in a disambiguated method name.
    pub fn trait_type_args(&self) -> Vec<String> {
        let Some(path) = &self.trait_path else {
            return Vec::new();
        };
        let Some(segment) = path.segments.last() else {
            return Vec::new();
        };
        let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
            return Vec::new();
        };
        args.args
            .iter()
            .filter_map(|a| match a {
                syn::GenericArgument::Type(ty) => Some(crate::name_map::map_type(ty)),
                _ => None,
            })
            .collect()
    }

    /// The bounds on each generic parameter, inline and `where` alike, in the
    /// TypeScript spelling emission writes into a class's type parameter list.
    /// Marker traits carry no shape and are left out.
    pub fn generic_bounds(&self) -> std::collections::HashMap<String, Vec<String>> {
        let mut out: std::collections::HashMap<String, Vec<String>> = Default::default();
        let mut add = |name: String, bound: &syn::TypeParamBound| {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                return;
            };
            let Some(seg) = trait_bound.path.segments.last() else {
                return;
            };
            let trait_name = seg.ident.to_string();
            if matches!(trait_name.as_str(), "Send" | "Sync" | "Sized" | "") {
                return;
            }
            let written = match &seg.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    let type_args: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|a| match a {
                            syn::GenericArgument::Type(ty) => Some(crate::name_map::map_type(ty)),
                            _ => None,
                        })
                        .collect();
                    if type_args.is_empty() {
                        trait_name
                    } else {
                        format!("{}<{}>", trait_name, type_args.join(", "))
                    }
                }
                _ => trait_name,
            };
            out.entry(name).or_default().push(written);
        };

        for param in &self.generics.params {
            if let syn::GenericParam::Type(t) = param {
                for bound in &t.bounds {
                    add(t.ident.to_string(), bound);
                }
            }
        }
        if let Some(where_clause) = &self.generics.where_clause {
            for pred in &where_clause.predicates {
                let syn::WherePredicate::Type(pt) = pred else {
                    continue;
                };
                let syn::Type::Path(p) = &pt.bounded_ty else {
                    continue;
                };
                let Some(name) = p.path.segments.last().map(|s| s.ident.to_string()) else {
                    continue;
                };
                for bound in &pt.bounds {
                    add(name.clone(), bound);
                }
            }
        }
        out
    }
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
