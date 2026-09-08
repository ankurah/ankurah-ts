//! Reading a `where` clause: what a declaration REQUIRES of its parameters.
//!
//! Split out of `build.rs`, which is the whole two-pass walk. One question is
//! asked here and three callers ask it — an `impl` block, a trait, and (FF4) a
//! struct whose field IS a bounded parameter, where the bound is the only thing
//! that says the field holds a cursor rather than an array.

use super::impls::Bound;
use super::resolve_type::{resolve_type, TypeEnv};
use crate::diag::DiagSink;
use crate::ty::Ty;

/// The `T: Trait` requirements an impl or trait writes, inline and in its
/// `where` clause alike.
pub fn resolve_bounds(generics: &syn::Generics, env: &TypeEnv, sink: &DiagSink) -> Vec<Bound> {
    let mut out = Vec::new();
    for param in &generics.params {
        let syn::GenericParam::Type(t) = param else {
            continue;
        };
        let subject = Ty::Param(t.ident.to_string());
        for bound in &t.bounds {
            push_bound(&subject, bound, env, sink, &mut out);
        }
    }
    let Some(where_clause) = &generics.where_clause else {
        return out;
    };
    for pred in &where_clause.predicates {
        let syn::WherePredicate::Type(pt) = pred else {
            continue;
        };
        let subject = match resolve_type(&pt.bounded_ty, env) {
            Ok(ty) => ty,
            Err(diag) => {
                sink.push(diag);
                continue;
            }
        };
        for bound in &pt.bounds {
            push_bound(&subject, bound, env, sink, &mut out);
        }
    }
    out
}

fn push_bound(
    subject: &Ty,
    bound: &syn::TypeParamBound,
    env: &TypeEnv,
    sink: &DiagSink,
    out: &mut Vec<Bound>,
) {
    let syn::TypeParamBound::Trait(t) = bound else {
        return;
    };
    // `T: ?Sized` lifts the implicit `Sized` requirement; it does not add one.
    // Reading it as a requirement made `impl<T: ?Sized> Deref for Arc<T>` — the
    // shape half the std surface is written in — demand a proof of the opposite
    // of what it says.
    if matches!(t.modifier, syn::TraitBoundModifier::Maybe(_)) {
        return;
    }
    match super::resolve_type::trait_ref(t, env) {
        Ok(trait_ref) => out.push(Bound {
            subject: subject.clone(),
            trait_ref,
        }),
        Err(diag) => {
            sink.push(diag);
            // A bound the engine could not read is still a bound. Dropping it
            // turned `impl<T: Display> ToString for T` into an impl with no
            // requirement at all, which then answered `to_string` on every type
            // there is. Standing it up against the written name — which nothing
            // declares, as far as this run could tell — makes it an obligation
            // nobody could decide, reported at each call.
            let segments: Vec<String> = t.path.segments.iter().map(|s| s.ident.to_string()).collect();
            let canonical = env.reg.canonical_path(env.module, &segments);
            if let Ok(id) = env.reg.foreign(&canonical) {
                out.push(Bound {
                    subject: subject.clone(),
                    trait_ref: crate::ty::TraitRef {
                        id,
                        args: Vec::new(),
                        bindings: Vec::new(),
                    },
                });
            }
        }
    }
}
