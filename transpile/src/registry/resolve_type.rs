//! `syn::Type` to `Ty` — the engine's only entry point for a written type.
//!
//! Everything the four crates write in type position resolves here, or the
//! attempt produces a diagnostic naming the span. Nothing falls back to a
//! stand-in type, and nothing a path does not actually name is silently
//! substituted for it.

use syn::spanned::Spanned;

use super::module::{Def, ModuleId};
use super::TypeRegistry;
use crate::diag::{Diag, DiagSink};
use crate::ty::{bind_params, ArrayLen, Prim, TraitRef, Ty, TypeId};

/// What a type is being read in: which module wrote it, which generic
/// parameters are in scope, and what `Self` means.
pub struct TypeEnv<'a> {
    pub reg: &'a TypeRegistry,
    pub module: ModuleId,
    pub params: &'a [String],
    pub self_ty: Option<&'a Ty>,
    pub sink: &'a DiagSink,
}

impl<'a> TypeEnv<'a> {
    pub fn new(reg: &'a TypeRegistry, module: ModuleId, sink: &'a DiagSink) -> Self {
        TypeEnv {
            reg,
            module,
            params: &[],
            self_ty: None,
            sink,
        }
    }

    pub fn with_params(mut self, params: &'a [String]) -> Self {
        self.params = params;
        self
    }

    pub fn with_self(mut self, self_ty: Option<&'a Ty>) -> Self {
        self.self_ty = self_ty;
        self
    }

    fn refuse(&self, span: proc_macro2::Span, message: impl Into<String>) -> Diag {
        Diag::at(&self.sink.file(), span, message)
    }
}

/// Where a path was written, which decides whether an undeclared name is
/// reported as a type or as a trait, and whether it is worth reporting at all.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Type,
    Trait,
}

pub fn resolve_type(ty: &syn::Type, env: &TypeEnv) -> Result<Ty, Diag> {
    match ty {
        syn::Type::Path(path) => resolve_path_type(path, env),

        syn::Type::Reference(r) => Ok(Ty::Ref {
            mutable: r.mutability.is_some(),
            inner: Box::new(resolve_type(&r.elem, env)?),
        }),

        syn::Type::Tuple(t) if t.elems.is_empty() => Ok(Ty::Unit),
        syn::Type::Tuple(t) => {
            let elems = t
                .elems
                .iter()
                .map(|e| resolve_type(e, env))
                .collect::<Result<_, _>>()?;
            Ok(Ty::Tuple(elems))
        }

        syn::Type::Slice(s) => Ok(Ty::Slice(Box::new(resolve_type(&s.elem, env)?))),

        syn::Type::Array(a) => Ok(Ty::Array {
            elem: Box::new(resolve_type(&a.elem, env)?),
            len: array_len(&a.len, env)?,
        }),

        syn::Type::Never(_) => Ok(Ty::Never),
        syn::Type::Infer(_) => Ok(Ty::Infer),
        syn::Type::Paren(p) => resolve_type(&p.elem, env),
        syn::Type::Group(g) => resolve_type(&g.elem, env),

        syn::Type::TraitObject(obj) => Ok(Ty::Dyn {
            traits: trait_refs(&obj.bounds, env)?,
        }),
        syn::Type::ImplTrait(it) => Ok(Ty::ImplTrait {
            bounds: trait_refs(&it.bounds, env)?,
        }),

        syn::Type::Ptr(_) => Err(env.refuse(ty.span(), "raw pointer type is not modelled")),
        syn::Type::BareFn(_) => Err(env.refuse(ty.span(), "function pointer type is not modelled")),
        syn::Type::Macro(_) => Err(env.refuse(ty.span(), "macro in type position is not expanded")),
        other => Err(env.refuse(other.span(), "type form is not modelled")),
    }
}

fn array_len(len: &syn::Expr, env: &TypeEnv) -> Result<ArrayLen, Diag> {
    match len {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(int),
            ..
        }) => int
            .base10_parse::<u64>()
            .map(ArrayLen::Lit)
            .map_err(|_| env.refuse(len.span(), "array length is not a plain integer")),
        syn::Expr::Path(path) if path.path.segments.len() == 1 => {
            Ok(ArrayLen::Named(path.path.segments[0].ident.to_string()))
        }
        other => Err(env.refuse(
            other.span(),
            "array length is not a literal or a named constant",
        )),
    }
}

fn resolve_path_type(path: &syn::TypePath, env: &TypeEnv) -> Result<Ty, Diag> {
    let span = path.span();
    let segments: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();

    // `<Vec<u8> as TryInto<Clock>>::Error` — a projection through a named trait,
    // whose own arguments matter and are kept.
    if let Some(qself) = &path.qself {
        let base = resolve_type(&qself.ty, env)?;
        let trait_ = match qself.position {
            0 => None,
            end => {
                let prefix: Vec<&syn::PathSegment> = path.path.segments.iter().take(end).collect();
                let last = prefix.last().copied();
                match last {
                    Some(seg) => Some(Box::new(path_trait_ref(&segments[..end], seg, env, span)?)),
                    None => None,
                }
            }
        };
        let name = segments.last().cloned().unwrap_or_default();
        return Ok(Ty::Assoc {
            base: Box::new(base),
            trait_,
            name,
        });
    }

    if segments.len() == 1 {
        let name = &segments[0];
        if name == "Self" {
            return match env.self_ty {
                Some(ty) => Ok(ty.clone()),
                None => Err(env.refuse(span, "`Self` written outside an impl")),
            };
        }
        if let Some(prim) = Prim::from_rust_name(name) {
            reject_arguments(&path.path, env, "a primitive type takes no arguments")?;
            return Ok(Ty::Prim(prim));
        }
        if name == "str" {
            reject_arguments(&path.path, env, "`str` takes no arguments")?;
            return Ok(Ty::Str);
        }
        if env.params.iter().any(|p| p == name) {
            reject_arguments(&path.path, env, "a generic parameter takes no arguments")?;
            return Ok(Ty::Param(name.clone()));
        }
    }

    // `Self::Error`, `T::Item` — a projection through the base's impls.
    if segments.len() > 1 {
        let head = &segments[0];
        let is_self = head == "Self";
        let is_param = env.params.iter().any(|p| p == head);
        if is_self || is_param {
            let base = if is_self {
                env.self_ty
                    .cloned()
                    .ok_or_else(|| env.refuse(span, "`Self` written outside an impl"))?
            } else {
                Ty::Param(head.clone())
            };
            let name = segments.last().cloned().unwrap_or_default();
            return Ok(Ty::Assoc {
                base: Box::new(base),
                trait_: None,
                name,
            });
        }
    }

    let args = generic_args(&path.path, env)?;
    resolve_named(&segments, args, env, span, Position::Type)
}

/// Resolve a written path to the type it names, expanding an alias where the
/// path names one.
fn resolve_named(
    segments: &[String],
    args: Vec<Ty>,
    env: &TypeEnv,
    span: proc_macro2::Span,
    position: Position,
) -> Result<Ty, Diag> {
    match env.reg.lookup_type(env.module, segments) {
        Err(err) => return Err(env.refuse(span, err.message)),
        Ok(Some(Def::Type(id))) => {
            let args = fill_defaults(id, args, env, span, segments)?;
            return Ok(Ty::Named { id, args });
        }
        Ok(Some(Def::Alias(id))) => return expand_alias(id, args, env, span),
        Ok(Some(Def::Value(_))) => {
            return Err(env.refuse(
                span,
                format!("`{}` is a value, not a type", segments.join("::")),
            ))
        }
        Ok(Some(Def::Module(_))) => {
            return Err(env.refuse(
                span,
                format!("`{}` is a module, not a type", segments.join("::")),
            ))
        }
        Ok(None) => {}
    }
    Ok(Ty::Named {
        id: undeclared(segments, env, span, position)?,
        args,
    })
}

/// A type nothing in reach declares. It keeps its written name and a distinct
/// identity, and is reported once.
fn undeclared(
    segments: &[String],
    env: &TypeEnv,
    span: proc_macro2::Span,
    position: Position,
) -> Result<TypeId, Diag> {
    let canonical = env.reg.canonical_path(env.module, segments);
    let id = env.reg.foreign(&canonical).map_err(|e| {
        env.refuse(
            span,
            format!("cannot name `{}`: {}", canonical.join("::"), e),
        )
    })?;
    let what = if position == Position::Trait {
        "trait"
    } else {
        "type"
    };
    env.sink.report_once(
        span,
        format!("no declaration for {} `{}`", what, canonical.join("::")),
    );
    env.reg.mark_reported(id);
    Ok(id)
}

/// Fill in the arguments the use site left unwritten, and refuse a path that
/// writes a number the declaration cannot take.
///
/// `HashMap<K, V, S = RandomState>` is declared with three parameters and
/// ankurah always writes two, so the third is filled in from the declaration —
/// which is what makes `impl<T, S: BuildHasher> Iterable<T> for HashSet<T, S>`
/// in the corpus unify against a written `HashSet<String>`. A path that supplies
/// too many, or too few with no default to fall back on, is not the type it
/// names: `std::fmt::Result` is not `Result<T, E>`.
fn fill_defaults(
    id: TypeId,
    mut args: Vec<Ty>,
    env: &TypeEnv,
    span: proc_macro2::Span,
    segments: &[String],
) -> Result<Vec<Ty>, Diag> {
    if !env.reg.is_system(id) {
        return Ok(args);
    }
    let Some(def) = env.reg.def(id) else {
        return Ok(args);
    };
    let declared = def.type_params.len();
    if args.len() == declared {
        return Ok(args);
    }
    if args.len() < declared {
        // A default is written in the declaring type's own parameters, and
        // std's are all closed (`RandomState`), so nothing has to be
        // substituted into them here.
        let missing: Vec<Option<Ty>> = def.param_defaults[args.len()..].to_vec();
        if missing.iter().all(|d| d.is_some()) {
            args.extend(missing.into_iter().map(|d| d.unwrap()));
            return Ok(args);
        }
    }
    Err(env.refuse(
        span,
        format!(
            "`{}` is declared with {} type argument(s) but {} written here",
            segments.join("::"),
            declared,
            args.len()
        ),
    ))
}

/// Expand a type alias in the module that declared it, then apply the arguments
/// written at the use site. A cycle stops with a diagnostic.
fn expand_alias(
    id: super::AliasId,
    mut args: Vec<Ty>,
    env: &TypeEnv,
    span: proc_macro2::Span,
) -> Result<Ty, Diag> {
    let Some(alias) = env.reg.alias(id) else {
        return Err(env.refuse(span, "type alias is missing its declaration"));
    };
    // `type Result<T, E = Error> = ..` written as `Result<()>` fills `E` in from
    // the alias's own declaration. Without it the expansion carried `E` out as a
    // loose parameter, and `anyhow::Result<()>` had an error type of nothing.
    for default in alias.param_defaults.iter().skip(args.len()) {
        let Some(rust_ty) = default else { break };
        let inner = TypeEnv::new(env.reg, alias.module, env.sink).with_params(&alias.type_params);
        args.push(resolve_type(rust_ty, &inner)?);
    }
    let expanded = env.reg.expanding_alias(id, || {
        let inner = TypeEnv::new(env.reg, alias.module, env.sink).with_params(&alias.type_params);
        resolve_type(&alias.rust_ty, &inner)
    });
    match expanded {
        None => Err(env.refuse(
            span,
            format!("type alias `{}` expands into itself", alias.name),
        )),
        Some(Err(diag)) => Err(diag),
        Some(Ok(ty)) => Ok(ty.substitute(&bind_params(&alias.type_params, &args))),
    }
}

/// Arguments written on a path. Anything that is not a type or a lifetime —
/// a const argument, an associated-type binding on a named type — is refused
/// rather than dropped.
fn generic_args(path: &syn::Path, env: &TypeEnv) -> Result<Vec<Ty>, Diag> {
    let Some(segment) = path.segments.last() else {
        return Ok(Vec::new());
    };
    match &segment.arguments {
        syn::PathArguments::None => Ok(Vec::new()),
        syn::PathArguments::AngleBracketed(args) => {
            let mut out = Vec::new();
            for arg in &args.args {
                match arg {
                    syn::GenericArgument::Type(ty) => out.push(resolve_type(ty, env)?),
                    syn::GenericArgument::Lifetime(_) => {}
                    syn::GenericArgument::Const(_) => {
                        return Err(env.refuse(
                            arg.span(),
                            "const generic argument is not modelled outside an array length",
                        ))
                    }
                    syn::GenericArgument::AssocType(assoc) => {
                        return Err(env.refuse(
                            arg.span(),
                            format!(
                                "associated type binding `{}` is only modelled on a trait bound",
                                assoc.ident
                            ),
                        ))
                    }
                    other => {
                        return Err(
                            env.refuse(other.span(), "generic argument form is not modelled")
                        )
                    }
                }
            }
            Ok(out)
        }
        // `Fn(A) -> R` as a bare type path drops the return type; the trait
        // bound form below keeps it, and that is the only form modelled.
        syn::PathArguments::Parenthesized(args) => Err(env.refuse(
            args.span(),
            "parenthesised argument list is only modelled on a trait bound",
        )),
    }
}

fn reject_arguments(path: &syn::Path, env: &TypeEnv, message: &str) -> Result<(), Diag> {
    let Some(segment) = path.segments.last() else {
        return Ok(());
    };
    match &segment.arguments {
        syn::PathArguments::None => Ok(()),
        other => Err(env.refuse(other.span(), message.to_string())),
    }
}

fn trait_refs(
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
    env: &TypeEnv,
) -> Result<Vec<TraitRef>, Diag> {
    let mut out = Vec::new();
    for bound in bounds {
        if let syn::TypeParamBound::Trait(t) = bound {
            out.push(trait_ref(t, env)?);
        }
    }
    Ok(out)
}

/// A trait bound. `Fn(A, B) -> R` is stored the way Rust desugars it: one
/// argument holding the tuple of inputs, and an `Output` binding.
pub fn trait_ref(bound: &syn::TraitBound, env: &TypeEnv) -> Result<TraitRef, Diag> {
    let span = bound.span();
    let segments: Vec<String> = bound
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    let Some(segment) = bound.path.segments.last() else {
        let id = undeclared(&segments, env, span, Position::Trait)?;
        return Ok(TraitRef {
            id,
            args: Vec::new(),
            bindings: Vec::new(),
        });
    };
    path_trait_ref(&segments, segment, env, span)
}

/// A trait named by a bare path, as an `impl Trait for T` block writes it.
pub fn trait_ref_of_path(path: &syn::Path, env: &TypeEnv) -> Result<TraitRef, Diag> {
    let span = path.span();
    let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    match path.segments.last() {
        Some(segment) => path_trait_ref(&segments, segment, env, span),
        None => Err(env.refuse(span, "an impl names no trait")),
    }
}

fn path_trait_ref(
    segments: &[String],
    segment: &syn::PathSegment,
    env: &TypeEnv,
    span: proc_macro2::Span,
) -> Result<TraitRef, Diag> {
    let id = match env.reg.lookup_type(env.module, segments) {
        Err(err) => return Err(env.refuse(span, err.message)),
        Ok(Some(Def::Type(id))) => id,
        Ok(Some(_)) | Ok(None) => undeclared(segments, env, span, Position::Trait)?,
    };

    match &segment.arguments {
        syn::PathArguments::None => Ok(TraitRef {
            id,
            args: Vec::new(),
            bindings: Vec::new(),
        }),
        syn::PathArguments::AngleBracketed(args) => {
            let mut tys = Vec::new();
            let mut bindings = Vec::new();
            for arg in &args.args {
                match arg {
                    syn::GenericArgument::Type(ty) => tys.push(resolve_type(ty, env)?),
                    syn::GenericArgument::AssocType(assoc) => {
                        bindings.push((assoc.ident.to_string(), resolve_type(&assoc.ty, env)?));
                    }
                    syn::GenericArgument::Lifetime(_) => {}
                    other => {
                        return Err(env.refuse(other.span(), "trait argument form is not modelled"))
                    }
                }
            }
            Ok(TraitRef {
                id,
                args: tys,
                bindings,
            })
        }
        syn::PathArguments::Parenthesized(args) => {
            let inputs: Vec<Ty> = args
                .inputs
                .iter()
                .map(|ty| resolve_type(ty, env))
                .collect::<Result<_, _>>()?;
            let input_ty = if inputs.is_empty() {
                Ty::Unit
            } else {
                Ty::Tuple(inputs)
            };
            let output = match &args.output {
                syn::ReturnType::Default => Ty::Unit,
                syn::ReturnType::Type(_, ty) => resolve_type(ty, env)?,
            };
            Ok(TraitRef {
                id,
                args: vec![input_ty],
                bindings: vec![("Output".to_string(), output)],
            })
        }
    }
}
