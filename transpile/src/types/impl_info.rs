//! An `impl` block as extraction read it, and what emission asks of one.
//!
//! Split out of `types.rs` so that neither file carries the other's weight: the
//! rest of `types.rs` is plain data read off `syn`, and this one is the only
//! extracted item with reasoning of its own — which methods a trait impl
//! supplies, and under what name each is written.

use super::FnInfo;
use crate::name_map::rust_spelling::rust_spelling;

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
    /// `const LIMIT: u32 = 5;` written INSIDE the impl, by name and span.
    ///
    /// G5: the port emits nothing for one, and `Self::LIMIT` then reads a
    /// member no class declares — `undefined`, silently, wherever it is used.
    /// Recorded here so the module that owns the file can say so.
    pub const_items: Vec<(String, proc_macro2::Span)>,
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
    pub fn generic_bounds(
        &self,
        declared: &[String],
    ) -> std::collections::HashMap<String, Vec<String>> {
        let mut out: std::collections::HashMap<String, Vec<String>> = Default::default();
        // The impl's OWN parameters, which the class may not declare:
        // `impl<I: Iterator<Item = R>, R> FilterIterator<I>` names the element
        // through `R`, and the class's generic list has only `I`. A spelling
        // that mentions such a name would name nothing at all where it is
        // written, so it is not carried and the bare trait name stands.
        let mine: Vec<String> = self
            .generics
            .params
            .iter()
            .filter_map(|p| match p {
                syn::GenericParam::Type(t) => Some(t.ident.to_string()),
                _ => None,
            })
            .collect();
        let nameable = |bound: &syn::TraitBound| {
            !mentions_a_param(&bound.path, &mine, declared)
        };
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
            // FF9: ONE spelling for a bound, wherever it is written. A class's
            // generics are merged from the impl blocks written for it, and this
            // is where those are spelled — so a `Fn` bound became a bare `Fn`
            // and an `Iterator<Item = V>` a bare `Iterator`, a name TypeScript's
            // own lib declares with two required arguments, while the same
            // bound on a FUNCTION came out `Iterable<V>`. The `Item = V` is an
            // associated binding, not a type argument, so the loop below never
            // saw it at all.
            if nameable(trait_bound) {
                if let Some(spelled) = crate::extract::bounds::bound_spelling(&trait_name, trait_bound) {
                    out.entry(name).or_default().push(spelled);
                    return;
                }
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

/// Does this bound's path mention a generic parameter of the IMPL that the
/// declaration being written does not declare?
///
/// FF9's guard, and the same rule `written_as_a_cursor` keeps for a signature:
/// a spelling has to be nameable WHERE IT IS WRITTEN.
fn mentions_a_param(path: &syn::Path, mine: &[String], declared: &[String]) -> bool {
    let mut found = false;
    for segment in &path.segments {
        let args = match &segment.arguments {
            syn::PathArguments::AngleBracketed(a) => a.args.iter().cloned().collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        for arg in args {
            let ty = match arg {
                syn::GenericArgument::Type(ty) => ty,
                syn::GenericArgument::AssocType(assoc) => assoc.ty,
                _ => continue,
            };
            let written = crate::name_map::rust_spelling::rust_spelling(&ty);
            found |= mine
                .iter()
                .any(|name| !declared.contains(name) && names_it(&written, name));
        }
    }
    found
}

/// Is `name` one of the identifiers in this written type?
fn names_it(written: &str, name: &str) -> bool {
    written
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == name)
}
