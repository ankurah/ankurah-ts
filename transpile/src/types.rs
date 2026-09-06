//! Data structures representing extracted Rust items
//!
//! Extraction keeps the `syn::Type` of everything it reads, because that is
//! what the type engine resolves. The TypeScript strings alongside them are
//! emission's business and are produced by `name_map`, never parsed back.

use crate::ty::Ty;
// A `use` item's shape lives beside the extraction that reads it.
pub(crate) use crate::extract::UseInfo;

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
    /// The ordinary functions a `#[cfg(test)] mod tests` declares beside its
    /// tests. `ankql/src/ast.rs`'s `nullify_columns` is one, and every one of
    /// that file's nine tests calls it: reading only the `#[test]` functions
    /// emitted a suite whose every case failed on `nullifyColumns is not
    /// defined`.
    pub test_helpers: Vec<FnInfo>,
    /// Raw TS declarations to emit at module level (e.g., thread_local → const)
    pub module_decls: Vec<String>,
    /// `mod x;` declarations with no body, and how each is written. Rust's
    /// `pub mod x;` is what puts `x`'s names in the crate's surface, so the
    /// emitted module index re-exports it; a private `mod x;` is reachable only
    /// from inside the crate and re-exports nothing.
    pub mod_decls: Vec<(String, VisInfo)>,
    /// Inline modules extracted as separate files.
    /// (module_name, RustFile) — emitted to parent_dir/module_name.ts
    pub inline_modules: Vec<(String, RustFile)>,
    /// Is this file the extracted body of a `#[cfg(test)] mod`?
    ///
    /// Its functions are DECLARED here — so a test that calls a helper of the
    /// same module resolves it — and translated and emitted as the parent's
    /// `test_functions`, which is where they belong. Without the declaration,
    /// `nullify_columns(..).unwrap()` in ankql's `ast.rs` answered "does not
    /// name a function here" and the `unwrap` was dropped as an identity, so
    /// every one of that file's nine tests compared a `Result` to a string.
    pub is_test_module: bool,
    /// The name of the inline module that is this file's `#[cfg(test)] mod`.
    ///
    /// It lives in `inline_modules` so that its declarations are registered,
    /// resolved and translated like any other module's — a test fixture is
    /// ordinary Rust — and it is named here so that EMISSION knows to write it
    /// into the `.test.ts` rather than into a `.ts` file of its own. The walk
    /// used to read only `syn::Item::Fn` out of it, so every fixture struct,
    /// impl, const and `use` was dropped and the emitted test named them
    /// anyway: 16 × TS2304 in core.
    pub test_module: Option<String>,
    /// Every field name something in this file assigns. Rust's `pub` means
    /// readable *and* writable, so a field anything writes cannot be emitted
    /// `readonly`. Read off the source while the bodies are still ASTs, because
    /// translation drops them once it has written the TypeScript.
    pub assigned_fields: std::collections::HashSet<String>,
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
            test_helpers: Vec::new(),
            test_module: None,
            is_test_module: false,
            module_decls: Vec::new(),
            mod_decls: Vec::new(),
            assigned_fields: std::collections::HashSet::new(),
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
    /// `HashMap<K, V, S = RandomState>` — what a parameter falls back to when
    /// the use site leaves it unwritten, positionally alongside `type_params`.
    pub param_defaults: Vec<Option<syn::Type>>,
    pub derives: Vec<String>,
    /// `#[serde(transparent)]` — the container has ONE field that is not
    /// skipped, and serde writes and reads that field's value with no wrapper
    /// around it. `core/src/property/value/entity_ref.rs`'s `Ref<T>` is a
    /// NAMED struct written this way, so the newtype rule alone did not catch
    /// it and the emitted JSON carried an `id` key and a `_phantom` beside it
    /// where serde writes the `EntityId` alone.
    pub serde_transparent: bool,
    /// Where the type's name is written, so a derive hook that cannot carry
    /// something over reports it at the declaration a reader has to open.
    pub span: proc_macro2::Span,
}

#[derive(Debug)]
pub struct FieldInfo {
    pub name: Option<String>,
    /// The field's name as Rust writes it. The emitted property is camelCase,
    /// and serde's JSON key is the Rust spelling, so the two are kept apart.
    /// `None` for a tuple field, which serde writes by position.
    pub rust_name: Option<String>,
    /// The Rust type as written — what the engine resolves, and what emission
    /// falls back to when it refused.
    pub rust_ty: syn::Type,
    /// The resolved type. `None` means the engine refused to name this type and
    /// filed a diagnostic; the fail-loud step turns that into an error.
    pub ty: Option<Ty>,
    pub is_pub: bool,
    /// `#[from]` on a `thiserror` variant field: the derive writes an
    /// `impl From<this field's type> for the enum`, and the registry has to
    /// know it as one so a `?` can find it.
    pub is_from: bool,
    /// `#[source]`, or the `#[from]` that implies one: this field is the error
    /// this one wraps, and `Error::source` answers it.
    pub is_source: bool,
    /// `#[serde(with = "json_as_bytes")]` — the module serde routes this
    /// field's two halves through, which changes the bytes on the wire. Read as
    /// the module's own name, so the codec can look up a hook for it by
    /// identity rather than expanding what the module writes.
    pub serde_with: Option<String>,
    /// `#[serde(skip)]` — the field is in neither format. serde writes nothing
    /// for it and reads it back as `Default::default()`.
    pub serde_skip: bool,
}

/// What a `thiserror` variant's `#[error(..)]` says.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorText {
    /// `#[error("no such {0}")]` — a format string the shared renderer writes.
    Format(String),
    /// `#[error(transparent)]` — this variant's text IS the wrapped error's, so
    /// `toString` forwards to it rather than saying anything of its own.
    Transparent,
}

#[derive(Debug)]
pub struct EnumInfo {
    pub name: String,
    pub is_pub: bool,
    pub vis: VisInfo,
    pub variants: Vec<VariantInfo>,
    pub generics: String,
    pub type_params: Vec<String>,
    pub param_defaults: Vec<Option<syn::Type>>,
    pub derives: Vec<String>,
    /// `#[serde(transparent)]`. See `StructInfo::serde_transparent`.
    pub serde_transparent: bool,
    /// Where the type's name is written. See `StructInfo::span`.
    pub span: proc_macro2::Span,
}

#[derive(Debug)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    /// True if the variant has `#[serde(other)]` — catch-all for unknown discriminants
    pub is_serde_other: bool,
    /// What this variant's `#[error(..)]` says, where thiserror's derive is
    /// what writes the type's `Display`. `None` where the variant carries no
    /// such attribute, or carries one this reader does not handle.
    pub error_text: Option<ErrorText>,
    /// Where the variant's name is written.
    pub span: proc_macro2::Span,
}

#[derive(Debug)]
pub struct TraitInfo {
    pub name: String,
    pub is_pub: bool,
    pub vis: VisInfo,
    /// `auto trait Send {}` — Rust decides these structurally, for every type
    /// that qualifies, with no impl written anywhere. Nothing can look one up
    /// in the impl table, so a bound on one is answered by the declaration.
    pub is_auto: bool,
    pub methods: Vec<FnInfo>,
    pub has_default_impls: bool,
    pub generics: String,
    pub type_params: Vec<String>,
    /// `trait Signal: Debug` — the traits an implementor must also implement,
    /// as written. A method reached on a `dyn Signal` may be declared on one of
    /// these rather than on `Signal` itself.
    pub supertraits: Vec<syn::TraitBound>,
    /// `type Item;` — the associated types each impl has to supply.
    /// `type IntoIter: Iterator<Item = Self::Item>;` — the names each impl has
    /// to supply a type for, each with what the trait says that type is good
    /// for. A projection no impl settles is still a type, and its declared
    /// bounds are the only thing that says what can be done with it (4.4a).
    pub assoc_types: Vec<(String, Vec<syn::TypeParamBound>)>,
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

#[derive(Clone)]
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
    /// Did translating this body write an R12 hole?
    ///
    /// Recorded by the lowering rather than read back out of `body_ts`: a body
    /// that mentions `unsupported(` for any other reason is not a body that
    /// refused a shape, and a method carrying a refusal is never dropped from
    /// the emitted class, because a refusal that is not in the file is not a
    /// refusal.
    pub body_has_hole: bool,
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
    /// The trait's type arguments as WRITTEN PATHS: `From<bincode::Error>` is
    /// `["bincode::Error"]`. `trait_type_args` gives the leaf alone, which is
    /// what names the emitted method most of the time and is not enough where
    /// two impls of one type convert from two `Error`s.
    pub fn trait_type_arg_paths(&self) -> Vec<String> {
        // Shares its spelling with `rust_source_path` below, because the
        // conversion names are looked up by it from two places.
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
                // The TypeScript spelling, with the module segments the source
                // wrote in front of it. The spelling is what every other
                // question about the type is asked of — is it a primitive, does
                // it carry arguments — and the qualifier is the one thing that
                // tells `bincode::Error` from `anyhow::Error`.
                syn::GenericArgument::Type(ty) => {
                    // A reference keeps its `&`. TypeScript erases it, so
                    // `From<Literal>` and `From<&Literal>` spell one signature
                    // — but they do NOT do the same thing with what they are
                    // given, and reading them as one string made the owned
                    // body run for a borrowed value and drop something its
                    // caller still owned.
                    let (inner, borrowed) = match ty {
                        syn::Type::Reference(r) => (&*r.elem, "&"),
                        other => (other, ""),
                    };
                    // The RUST leaf, not the TypeScript spelling. R8: a
                    // contested conversion is qualified by the source type as
                    // Rust wrote it, and `i64`, `i32` and `f64` are all
                    // `number` — read through TypeScript, three impls looked
                    // like one and two of them were never emitted. A leaf that
                    // carries arguments has no name to give either way, so it
                    // keeps the spelling that shows what they are.
                    let spelled = match inner {
                        syn::Type::Path(p)
                            if p.path
                                .segments
                                .last()
                                .is_some_and(|s| matches!(s.arguments, syn::PathArguments::None)) =>
                        {
                            p.path
                                .segments
                                .last()
                                .map(|s| s.ident.to_string())
                                .unwrap_or_default()
                        }
                        // A leaf that carries ARGUMENTS keeps its Rust spelling
                        // too. Written in TypeScript, `Vec<u32>` and `Vec<i32>`
                        // are both `number[]` — so two impls with two different
                        // bodies were one identity, and the second was dropped
                        // with no diagnostic. R8's rule is the Rust source, and
                        // it reaches all the way down.
                        other => rust_spelling(other),
                    };
                    let qualifier = match inner {
                        syn::Type::Path(p) if p.path.segments.len() > 1 => p
                            .path
                            .segments
                            .iter()
                            .take(p.path.segments.len() - 1)
                            .map(|s| s.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::"),
                        _ => String::new(),
                    };
                    Some(if qualifier.is_empty() {
                        format!("{}{}", borrowed, spelled)
                    } else {
                        format!("{}{}::{}", borrowed, qualifier, spelled)
                    })
                }
                _ => None,
            })
            .collect()
    }

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
pub struct TypeAliasInfo {
    pub name: String,
    pub ty: String,
    pub rust_ty: syn::Type,
    pub is_pub: bool,
    pub vis: VisInfo,
    /// Generic parameter names the alias declares.
    pub type_params: Vec<String>,
    /// `type Result<T, E = Error> = ..` — what a parameter the use site leaves
    /// unwritten falls back to, positionally alongside `type_params`.
    pub param_defaults: Vec<Option<syn::Type>>,
}

#[derive(Debug, Clone)]
pub struct ConstInfo {
    pub name: String,
    pub ty: String,
    pub rust_ty: Option<syn::Type>,
    pub is_pub: bool,
    pub vis: VisInfo,
    /// What the const is, as Rust wrote it. Carried so that emission can
    /// translate it: a `ConstInfo` used to hold the const's TYPE and nothing
    /// else, and every module-level const came out `undefined as any` — the
    /// word list `human_id` indexes, the tag byte every JSON value in an index
    /// key is written with, the system collection's name.
    pub init: Option<syn::Expr>,
    /// The translated initialiser, filled by the same pass that translates a
    /// function body and read by codegen.
    pub init_ts: Option<String>,
    /// `static mut NAME` — a global the program writes to, which TypeScript
    /// spells `let` rather than `const`.
    pub mutable: bool,
    /// Is this a `static` rather than a `const`?
    ///
    /// Rust means two different things by them, and the port has to keep them
    /// apart. A `static` is ONE place that lives as long as the program, and a
    /// `static COUNTER: AtomicUsize` is written through — which, since an atomic
    /// IS its value here, is an assignment to the module binding, and a `const`
    /// binding throws on one. A `const` is a value INLINED at each use: two uses
    /// of a non-`Copy` const are two values, and binding both to one module
    /// object gave them one identity, one mutation and one release.
    pub is_static: bool,
    /// Is every use of this name a FRESH value, so that the emitted name is a
    /// function each use calls? See `ValueDef::fresh_at_each_use`.
    pub fresh_at_each_use: bool,
}

impl FieldInfo {
    /// The TypeScript type this field is emitted with.
    ///
    /// Produced from the resolved type. When the engine refused this type it
    /// filed a diagnostic, and emission keeps the syntactic mapping so that
    /// output stays comparable step to step; the fail-loud step removes the
    /// second arm along with every other fallback.
    /// Where the field's type is written, so a gap found at emission is filed
    /// at the line a reader has to open.
    pub fn rust_ty_span(&self) -> proc_macro2::Span {
        syn::spanned::Spanned::span(&self.rust_ty)
    }

    pub fn ts_ty(&self, reg: &crate::registry::TypeRegistry) -> String {
        // A resolved type has no memory of the alias it was written as, so
        // writing the field from it turns `Listener` into the `Arc<dyn Fn(T)>`
        // the alias stands for. The port emits the alias, and the alias is what
        // the source said — under a reference and inside a wrapper too.
        if crate::name_map::names_an_alias(reg, &self.rust_ty) {
            return crate::name_map::map_type(&self.rust_ty);
        }
        match &self.ty {
            Some(ty) => crate::name_map::map_ty(reg, ty),
            None => crate::name_map::map_type(&self.rust_ty),
        }
    }
}

pub use crate::name_map::rust_spelling::{rust_source_path, rust_spelling};
