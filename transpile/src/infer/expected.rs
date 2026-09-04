//! What the surrounding code says an expression's type has to be.
//!
//! For: Rust decides a great deal from context that the written expression does
//! not carry. `let n: u8 = 1` is a `u8` and not the `i32` a bare `1` defaults
//! to; `s.parse()?` behind `let id: EntityId` parses an `EntityId`; `|x| ..`
//! passed to `f(g: impl Fn(u32))` binds `x` to a `u32`. None of that is
//! readable from the expression alone, so the position hands the expression a
//! type to be, and this file holds the few questions the translator asks of
//! such a type.
//!
//! The propagation is one level deep and never guesses (spec 4.6): an
//! expectation either says something definite about the expression under it or
//! it is dropped.

use crate::registry::TypeRegistry;
use crate::ty::{Prim, TraitRef, Ty};

/// The inputs and output of a callable an expected type describes.
///
/// `Fn(A, B) -> R` is stored desugared — one argument holding the tuple of
/// inputs and an `Output` binding — so this is that pair read back out, with
/// the tuple opened up into the parameter list a closure is written with.
#[derive(Debug, Clone, PartialEq)]
pub struct FnShape {
    pub inputs: Vec<Ty>,
    pub output: Ty,
}

/// The `Fn`-family trait ids, looked up once per question rather than carried
/// around: the three of them are what every callable bound names.
fn fn_traits(reg: &TypeRegistry) -> Vec<crate::ty::TypeId> {
    ["std::ops::Fn", "std::ops::FnMut", "std::ops::FnOnce"]
        .iter()
        .filter_map(|path| reg.system_type(path))
        .collect()
}

/// The callable an expected type describes, where it describes one.
///
/// A closure's parameter types are written nowhere in the source that passes
/// it, so they come from here: the `impl Fn(T)` a parameter declares, the `dyn
/// Fn(T)` a `Box`, `Arc` or `Rc` holds, or the bound a type parameter in scope
/// carries. Everything else has no callable in it and answers `None`.
pub fn fn_shape(
    reg: &TypeRegistry,
    ty: &Ty,
    param_bounds: &[(String, TraitRef)],
) -> Option<FnShape> {
    let callable = fn_traits(reg);
    from_ty(reg, ty, param_bounds, &callable, 0)
}

/// Wrappers a callable is written inside. Each is transparent to the question
/// "what does calling this do?": `Box<dyn Fn(T)>` is called exactly as the
/// `dyn Fn(T)` is.
const CALLABLE_WRAPPERS: [&str; 3] = [
    "std::boxed::Box",
    "std::sync::Arc",
    "std::rc::Rc",
];

fn from_ty(
    reg: &TypeRegistry,
    ty: &Ty,
    param_bounds: &[(String, TraitRef)],
    callable: &[crate::ty::TypeId],
    depth: usize,
) -> Option<FnShape> {
    // Four wrappers deep is more than any written type in the corpus and stops
    // a cycle in a declaration from spinning here.
    if depth > 4 {
        return None;
    }
    match ty {
        Ty::Ref { inner, .. } => from_ty(reg, inner, param_bounds, callable, depth + 1),
        Ty::ImplTrait { bounds } => first_callable(bounds, callable),
        Ty::Dyn { traits } => first_callable(traits, callable),
        // `F` where the enclosing signature wrote `F: Fn(T) -> R`. The bound is
        // the only thing that says what `F` can do.
        Ty::Param(name) => {
            let bounds: Vec<TraitRef> = param_bounds
                .iter()
                .filter(|(param, _)| param == name)
                .map(|(_, bound)| bound.clone())
                .collect();
            first_callable(&bounds, callable)
        }
        Ty::Named { id, args } => {
            let inner = args.first()?;
            CALLABLE_WRAPPERS
                .iter()
                .any(|path| reg.system_type(path) == Some(*id))
                .then(|| from_ty(reg, inner, param_bounds, callable, depth + 1))
                .flatten()
        }
        _ => None,
    }
}

/// The first `Fn`, `FnMut` or `FnOnce` in a bound list, read back into a
/// parameter list and a return type.
fn first_callable(bounds: &[TraitRef], callable: &[crate::ty::TypeId]) -> Option<FnShape> {
    let bound = bounds.iter().find(|b| callable.contains(&b.id))?;
    let inputs = match bound.args.first() {
        // `Fn()` writes its empty input list as the unit type, and `Fn(A)`
        // writes a one-element tuple; both are the tuple Rust desugars to.
        Some(Ty::Tuple(elems)) => elems.clone(),
        Some(Ty::Unit) | None => Vec::new(),
        Some(other) => vec![other.clone()],
    };
    let output = bound
        .bindings
        .iter()
        .find(|(name, _)| name == "Output")
        .map(|(_, ty)| ty.clone())
        .unwrap_or(Ty::Unit);
    Some(FnShape { inputs, output })
}

/// The callable an expected type describes, following one blanket impl where
/// the type itself names no callable.
///
/// For: `Ref::listen<L>(listener: L) where L: IntoBroadcastListener<T>` says
/// nothing about calling `L`. What says it is the blanket impl the trait
/// carries — `impl<F: Fn(T)> IntoBroadcastListener<T> for F` — which accepts a
/// closure and no other shape, so a closure written at that argument is a
/// closure of exactly that bound. This is the reverse of the deferred
/// obligation the resolution files: the obligation asks whether `L` is an `Fn`,
/// and a closure standing there is the answer.
///
/// One hop only, and only where exactly one blanket impl of the bound has a
/// callable bound of its own. Two would be a choice, and this makes none.
pub fn fn_shape_through_impls(
    probe: &crate::registry::Probe<'_>,
    reg: &TypeRegistry,
    ty: &Ty,
    param_bounds: &[(String, TraitRef)],
) -> Option<FnShape> {
    if let Some(direct) = fn_shape(reg, ty, param_bounds) {
        return Some(direct);
    }
    let callable = fn_traits(reg);
    let mut found: Option<FnShape> = None;
    for bound in bounds_on(ty, param_bounds) {
        if callable.contains(&bound.id) {
            continue;
        }
        for id in reg.impls().of_trait(bound.id) {
            let def = reg.impl_def(*id);
            if !def.is_blanket() {
                continue;
            }
            let Some(implemented) = def.trait_ref.as_ref() else {
                continue;
            };
            let Some(subst) = def.match_written_args(implemented, &bound) else {
                continue;
            };
            // The impl's self type is its own parameter, and what that
            // parameter is required to be is the callable a closure here has
            // to have.
            let Ty::Param(subject) = &def.self_ty else {
                continue;
            };
            let callable_bounds: Vec<TraitRef> = def
                .bounds
                .iter()
                .filter(|b| matches!(&b.subject, Ty::Param(name) if name == subject))
                .map(|b| b.trait_ref.substitute(&subst))
                .collect();
            let Some(shape) = first_callable(&callable_bounds, &callable) else {
                continue;
            };
            let shape = FnShape {
                inputs: shape.inputs.iter().map(|t| probe.normalize(t)).collect(),
                output: probe.normalize(&shape.output),
            };
            // Two blanket impls that both accept a closure would be a choice
            // between two signatures, and there is no rule here to make it.
            if found.as_ref().is_some_and(|first| *first != shape) {
                return None;
            }
            found = Some(shape);
        }
    }
    found
}

/// The traits a type is required to implement here: written on a `dyn` or an
/// `impl Trait`, or declared on the parameter by whatever brought it into
/// scope.
fn bounds_on(ty: &Ty, param_bounds: &[(String, TraitRef)]) -> Vec<TraitRef> {
    match ty.peel_refs() {
        Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => traits.clone(),
        Ty::Param(name) => param_bounds
            .iter()
            .filter(|(param, _)| param == name)
            .map(|(_, bound)| bound.clone())
            .collect(),
        _ => Vec::new(),
    }
}

/// The integer width an expectation imposes on a literal written without a
/// suffix.
///
/// Rust gives such a literal the type the position wants and falls back to
/// `i32` only when nothing wants anything. The wire format is width-sensitive,
/// so taking the default where the position said `u8` writes the wrong number
/// of bytes.
pub fn integer_width(expected: &Ty) -> Option<Prim> {
    match expected.peel_refs() {
        Ty::Prim(prim) if prim.is_integer() => Some(*prim),
        _ => None,
    }
}

/// The float width an expectation imposes, on the same terms.
pub fn float_width(expected: &Ty) -> Option<Prim> {
    match expected.peel_refs() {
        Ty::Prim(prim @ (Prim::F32 | Prim::F64)) => Some(*prim),
        _ => None,
    }
}

/// Is a sequence expected here a sequence of bytes?
///
/// `Vec<u8>`, `[u8; N]`, `&[u8]` and `Box<[u8]>` are one runtime type in the
/// port — a `Uint8Array` — and a literal written into one of those positions
/// has to be emitted as that rather than as a JavaScript array. The two sides
/// compare unequal otherwise, which is what `assert_eq!(bytes, [1, 2, 3])` was
/// failing on.
pub fn expects_bytes(reg: &TypeRegistry, expected: &Ty) -> bool {
    fn is_u8(ty: &Ty) -> bool {
        matches!(ty, Ty::Prim(Prim::U8))
    }
    match expected.peel_refs() {
        Ty::Slice(elem) => is_u8(elem),
        Ty::Array { elem, .. } => is_u8(elem),
        Ty::Named { id, args } => {
            let holds_u8 = args.first().is_some_and(|arg| match arg {
                Ty::Slice(elem) => is_u8(elem),
                other => is_u8(other),
            });
            holds_u8
                && ["std::vec::Vec", "std::boxed::Box"]
                    .iter()
                    .any(|path| reg.system_type(path) == Some(*id))
        }
        _ => false,
    }
}

/// The written type with its `_` holes filled from the expectation.
///
/// `let v: Vec<_> = it.collect()` and `it.collect::<Vec<_>>()` both leave a
/// hole the position can close. Where the two shapes do not line up the written
/// type stands as it was: an expectation is a hint about this position, not a
/// claim that overrides what the source said.
pub fn fill_infer(written: &Ty, expected: &Ty) -> Ty {
    match (written, expected) {
        (Ty::Infer, filled) => filled.clone(),
        (Ty::Named { id, args }, Ty::Named { id: want, args: fills }) if id == want => Ty::Named {
            id: *id,
            args: zip_fill(args, fills),
        },
        (Ty::Tuple(elems), Ty::Tuple(fills)) => Ty::Tuple(zip_fill(elems, fills)),
        (Ty::Slice(elem), Ty::Slice(fill)) => Ty::Slice(Box::new(fill_infer(elem, fill))),
        (
            Ty::Array { elem, len },
            Ty::Array {
                elem: fill,
                len: _,
            },
        ) => Ty::Array {
            elem: Box::new(fill_infer(elem, fill)),
            len: len.clone(),
        },
        (
            Ty::Ref { mutable, inner },
            Ty::Ref {
                mutable: _,
                inner: fill,
            },
        ) => Ty::Ref {
            mutable: *mutable,
            inner: Box::new(fill_infer(inner, fill)),
        },
        // A reference is erased in emission, so an expectation written behind
        // one still fills a hole in front of it.
        (written, Ty::Ref { inner, .. }) => fill_infer(written, inner),
        (written, _) => written.clone(),
    }
}

fn zip_fill(written: &[Ty], expected: &[Ty]) -> Vec<Ty> {
    written
        .iter()
        .enumerate()
        .map(|(i, ty)| match expected.get(i) {
            Some(fill) => fill_infer(ty, fill),
            None => ty.clone(),
        })
        .collect()
}

/// Is every part of this type one the engine can name here?
///
/// A parameter the enclosing signature declared — the `T` of `impl<T>
/// Broadcast<T>` — is a real type and stays. A parameter belonging to somebody
/// else's signature that nothing bound — the `B` of `Iterator::map`, the `U` of
/// `TryInto::try_into` — is a question, and answering with it would state as
/// fact what nobody has decided.
pub fn is_settled(ty: &Ty, in_scope: &[String]) -> bool {
    !has_infer(ty) && !holds_unbound(ty, in_scope)
}

fn holds_unbound(ty: &Ty, in_scope: &[String]) -> bool {
    let unbound = |name: &String| name != "Self" && !in_scope.iter().any(|p| p == name);
    match ty {
        Ty::Param(name) => unbound(name),
        Ty::Named { args, .. } => args.iter().any(|a| holds_unbound(a, in_scope)),
        Ty::Tuple(elems) => elems.iter().any(|e| holds_unbound(e, in_scope)),
        Ty::Slice(elem) | Ty::Array { elem, .. } => holds_unbound(elem, in_scope),
        Ty::Ref { inner, .. } => holds_unbound(inner, in_scope),
        Ty::Assoc { base, .. } => holds_unbound(base, in_scope),
        Ty::ImplTrait { bounds } | Ty::Dyn { traits: bounds } => bounds.iter().any(|b| {
            b.args.iter().any(|a| holds_unbound(a, in_scope))
                || b.bindings.iter().any(|(_, t)| holds_unbound(t, in_scope))
        }),
        _ => false,
    }
}

/// Does this type still hold a `_` the position never closed?
pub fn has_infer(ty: &Ty) -> bool {
    match ty {
        Ty::Infer => true,
        Ty::Named { args, .. } => args.iter().any(has_infer),
        Ty::Tuple(elems) => elems.iter().any(has_infer),
        Ty::Slice(elem) | Ty::Array { elem, .. } => has_infer(elem),
        Ty::Ref { inner, .. } => has_infer(inner),
        Ty::Assoc { base, .. } => has_infer(base),
        _ => false,
    }
}

/// The element type a sequence expectation names, for a literal written into
/// one: `Vec<u8>` and `[u8; 4]` both say `u8` about every element.
pub fn element_of(reg: &TypeRegistry, expected: &Ty) -> Option<Ty> {
    match expected.peel_refs() {
        Ty::Slice(elem) | Ty::Array { elem, .. } => Some((**elem).clone()),
        Ty::Named { id, args } => {
            let seq = ["std::vec::Vec", "std::collections::VecDeque"]
                .iter()
                .any(|path| reg.system_type(path) == Some(*id));
            seq.then(|| args.first().cloned()).flatten()
        }
        _ => None,
    }
}
