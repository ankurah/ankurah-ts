//! Matching a written type against an impl's pattern.
//!
//! `impl<T> Signal for Arc<Inner<T>>` says nothing until something asks whether
//! a particular receiver is one of those. Unification answers that: it walks the
//! pattern and the concrete type together, binds each of the impl's own
//! parameters to whatever stands at its position, and refuses when the shapes
//! disagree.
//!
//! For impl matching only the pattern carries unknowns. A `Param` the impl
//! declared is a variable; every other `Param` — one the calling function
//! declared, or `Self` inside a trait — is rigid and matches only itself. That
//! is what an impl actually means: `impl<T> Foo for Vec<T>` applies to every
//! `Vec`, while `impl Foo for Vec<T>` written inside `impl<T>`'s body applies
//! to that `T`.
//!
//! The walk itself is shared with the body solver, which owns unknowns on both
//! sides (`super::vars`), so each `Ty` variant is compared in one place.

use super::def::{InferId, TraitRef, Ty};
use super::subst::Subst;

/// Why a pattern and a concrete type could not be matched.
#[derive(Debug, Clone, PartialEq)]
pub enum Mismatch {
    /// The two types are built differently: `Vec<T>` against `HashMap<K, V>`,
    /// `&T` against `T`, a tuple of two against a tuple of three.
    Shape { pattern: Ty, concrete: Ty },
    /// One parameter was asked to stand for two different types, which is what
    /// `impl Foo for (T, T)` does when handed `(u8, u16)`.
    Conflict {
        param: String,
        bound: Ty,
        found: Ty,
    },
    /// Binding would make the parameter contain itself, as `T = Vec<T>` does.
    /// Rust has no infinite types, so this is always a failed match.
    Occurs { param: String, ty: Ty },
    /// The parameter would stand for a type nothing has worked out yet. No
    /// solver here ever discharges that, so an impl chosen on the strength of it
    /// would have been chosen on no evidence.
    Unresolved { param: String },
    /// Binding would make an inference variable contain itself, as `?0 =
    /// Vec<?0>` does. There is no such type, so the constraint has no solution.
    VarOccurs { var: InferId, ty: Ty },
    /// An unsuffixed literal was asked to be a type its kind excludes: Rust's
    /// `{integer}` standing for a float, or the other way round. The
    /// restriction is a fact about the program, so this is a contradiction even
    /// while the variable itself stands for nothing yet.
    Kind { var: InferId, ty: Ty },
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mismatch::Shape { .. } => write!(f, "the shapes differ"),
            Mismatch::Conflict { param, .. } => {
                write!(f, "`{}` would have to be two different types", param)
            }
            Mismatch::Occurs { param, .. } => write!(f, "`{}` would contain itself", param),
            Mismatch::Unresolved { param } => {
                write!(f, "`{}` would stand for a type that is not known yet", param)
            }
            Mismatch::VarOccurs { .. } => {
                write!(f, "an inferred type would contain itself")
            }
            Mismatch::Kind { .. } => {
                write!(f, "a literal cannot be both an integer and a float")
            }
        }
    }
}

/// What a unification walk treats as an unknown, and where it records the
/// answer. The structural walk is shared, so a new `Ty` variant is answered in
/// one place rather than once per unifier.
pub trait Unknowns {
    /// What this side stands for now, when a binding already made says so.
    /// `None` means it stands for itself. Every unknown the walk asks about
    /// passes through here, which is where a table records what a constraint
    /// stands on.
    fn follow(&mut self, ty: &Ty) -> Option<Ty> {
        let _ = ty;
        None
    }

    /// Record an answer when either side is one of this walk's unknowns.
    /// `None` means neither is, so the structural walk continues.
    fn bind(&mut self, a: &Ty, b: &Ty) -> Option<Result<(), Mismatch>>;
}

/// The parameters an impl declared, recorded into a `Subst`. Only the pattern
/// side carries unknowns; the receiver it is matched against is already fixed.
struct ImplParams<'a> {
    vars: &'a [String],
    subst: &'a mut Subst,
}

impl Unknowns for ImplParams<'_> {
    fn bind(&mut self, a: &Ty, b: &Ty) -> Option<Result<(), Mismatch>> {
        match a {
            Ty::Param(name) if self.vars.iter().any(|v| v == name) => {
                Some(bind_param(name, b, self.subst))
            }
            _ => None,
        }
    }
}

/// Match `concrete` against `pattern`, binding the pattern's variables.
///
/// `vars` names the parameters the pattern's owner declared. `subst` accumulates
/// the bindings and is left partly filled when the match fails, so a caller that
/// might retry hands in a fresh one.
pub fn unify(
    vars: &[String],
    pattern: &Ty,
    concrete: &Ty,
    subst: &mut Subst,
) -> Result<(), Mismatch> {
    unify_with(&mut ImplParams { vars, subst }, pattern, concrete)
}

/// Walk two types together, asking `unknowns` at every node before comparing
/// shapes. Each side is first rewritten through whatever is already bound, so a
/// variable is compared as the type it stands for.
pub fn unify_with<U: Unknowns>(
    unknowns: &mut U,
    pattern: &Ty,
    concrete: &Ty,
) -> Result<(), Mismatch> {
    if let Some(followed) = unknowns.follow(pattern) {
        return unify_with(unknowns, &followed, concrete);
    }
    if let Some(followed) = unknowns.follow(concrete) {
        return unify_with(unknowns, pattern, &followed);
    }
    if let Some(answer) = unknowns.bind(pattern, concrete) {
        return answer;
    }
    match (pattern, concrete) {
        // A rigid parameter stands for one specific type, so only that same
        // parameter matches it.
        (Ty::Param(a), Ty::Param(b)) if a == b => Ok(()),

        (Ty::Named { id: a, args: xs }, Ty::Named { id: b, args: ys })
            if a == b && xs.len() == ys.len() =>
        {
            for (x, y) in xs.iter().zip(ys) {
                unify_with(unknowns, x, y)?;
            }
            Ok(())
        }

        (
            Ty::Ref {
                mutable: a,
                inner: x,
            },
            Ty::Ref {
                mutable: b,
                inner: y,
            },
        ) if a == b => unify_with(unknowns, x, y),

        (Ty::Tuple(xs), Ty::Tuple(ys)) if xs.len() == ys.len() => {
            for (x, y) in xs.iter().zip(ys) {
                unify_with(unknowns, x, y)?;
            }
            Ok(())
        }

        (Ty::Slice(x), Ty::Slice(y)) => unify_with(unknowns, x, y),

        (Ty::Array { elem: x, len: n }, Ty::Array { elem: y, len: m }) if n == m => {
            unify_with(unknowns, x, y)
        }

        (Ty::Dyn { traits: xs }, Ty::Dyn { traits: ys }) if xs.len() == ys.len() => {
            for (x, y) in xs.iter().zip(ys) {
                unify_trait(unknowns, x, y)?;
            }
            Ok(())
        }

        (Ty::ImplTrait { bounds: xs }, Ty::ImplTrait { bounds: ys }) if xs.len() == ys.len() => {
            for (x, y) in xs.iter().zip(ys) {
                unify_trait(unknowns, x, y)?;
            }
            Ok(())
        }

        (
            Ty::Assoc {
                base: xb,
                trait_: xt,
                name: xn,
            },
            Ty::Assoc {
                base: yb,
                trait_: yt,
                name: yn,
            },
        ) if xn == yn => {
            match (xt, yt) {
                (Some(x), Some(y)) => unify_trait(unknowns, x, y)?,
                (None, None) => {}
                _ => {
                    return Err(Mismatch::Shape {
                        pattern: pattern.clone(),
                        concrete: concrete.clone(),
                    })
                }
            }
            unify_with(unknowns, xb, yb)
        }

        (Ty::Prim(a), Ty::Prim(b)) if a == b => Ok(()),
        (Ty::Str, Ty::Str) | (Ty::Unit, Ty::Unit) | (Ty::Never, Ty::Never) => Ok(()),
        // `_` stands for a type nothing has worked out yet. It matches the same
        // hole and nothing else: treating it as a wildcard would make every impl
        // a candidate and turn an unresolved type into an ambiguity report.
        (Ty::Infer, Ty::Infer) => Ok(()),

        _ => Err(Mismatch::Shape {
            pattern: pattern.clone(),
            concrete: concrete.clone(),
        }),
    }
}

fn unify_trait<U: Unknowns>(
    unknowns: &mut U,
    pattern: &TraitRef,
    concrete: &TraitRef,
) -> Result<(), Mismatch> {
    if pattern.id != concrete.id || pattern.args.len() != concrete.args.len() {
        return Err(Mismatch::Shape {
            pattern: Ty::Dyn {
                traits: vec![pattern.clone()],
            },
            concrete: Ty::Dyn {
                traits: vec![concrete.clone()],
            },
        });
    }
    for (x, y) in pattern.args.iter().zip(&concrete.args) {
        unify_with(unknowns, x, y)?;
    }
    for (name, x) in &pattern.bindings {
        match concrete.bindings.iter().find(|(n, _)| n == name) {
            Some((_, y)) => unify_with(unknowns, x, y)?,
            None => {
                return Err(Mismatch::Shape {
                    pattern: x.clone(),
                    concrete: Ty::Infer,
                })
            }
        }
    }
    Ok(())
}

fn bind_param(param: &str, ty: &Ty, subst: &mut Subst) -> Result<(), Mismatch> {
    if let Some(existing) = subst.get(param) {
        return if existing == ty {
            Ok(())
        } else {
            Err(Mismatch::Conflict {
                param: param.to_string(),
                bound: existing.clone(),
                found: ty.clone(),
            })
        };
    }
    if ty.mentions_param(param) {
        return Err(Mismatch::Occurs {
            param: param.to_string(),
            ty: ty.clone(),
        });
    }
    // A written `_` has no solver behind it, so an impl selected on one would
    // be selected on nothing. An inference variable is different: the body
    // solver may still settle it, and a tie between impls is reported as an
    // ambiguity rather than picked.
    if ty.mentions_infer() {
        return Err(Mismatch::Unresolved {
            param: param.to_string(),
        });
    }
    subst.insert(param.to_string(), ty.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::{Prim, TypeId};

    fn named(id: u32, args: Vec<Ty>) -> Ty {
        Ty::Named {
            id: TypeId(id),
            args,
        }
    }

    fn param(name: &str) -> Ty {
        Ty::Param(name.into())
    }

    #[test]
    fn a_declared_parameter_binds_to_whatever_stands_at_its_position() {
        // impl<T> Signal for Arc<Inner<T>>, handed Arc<Inner<u32>>.
        let pattern = named(1, vec![named(2, vec![param("T")])]);
        let concrete = named(1, vec![named(2, vec![Ty::Prim(Prim::U32)])]);
        let mut subst = Subst::new();
        assert_eq!(unify(&["T".into()], &pattern, &concrete, &mut subst), Ok(()));
        assert_eq!(subst.get("T"), Some(&Ty::Prim(Prim::U32)));
    }

    #[test]
    fn a_parameter_the_pattern_did_not_declare_matches_only_itself() {
        let mut subst = Subst::new();
        assert_eq!(unify(&[], &param("T"), &param("T"), &mut subst), Ok(()));
        assert!(subst.is_empty(), "nothing was bound");

        let mut subst = Subst::new();
        assert!(matches!(
            unify(&[], &param("T"), &Ty::Prim(Prim::U8), &mut subst),
            Err(Mismatch::Shape { .. })
        ));
    }

    #[test]
    fn one_parameter_cannot_stand_for_two_types() {
        // impl<T> Foo for (T, T), handed (u8, u16).
        let pattern = Ty::Tuple(vec![param("T"), param("T")]);
        let concrete = Ty::Tuple(vec![Ty::Prim(Prim::U8), Ty::Prim(Prim::U16)]);
        let mut subst = Subst::new();
        assert!(matches!(
            unify(&["T".into()], &pattern, &concrete, &mut subst),
            Err(Mismatch::Conflict { .. })
        ));
    }

    #[test]
    fn a_parameter_may_not_contain_itself() {
        // T = Vec<T> has no solution, and looping to find one is the bug this
        // check exists to prevent.
        let mut subst = Subst::new();
        let err = unify(
            &["T".into()],
            &param("T"),
            &named(1, vec![param("T")]),
            &mut subst,
        );
        assert!(matches!(err, Err(Mismatch::Occurs { .. })), "{:?}", err);
    }

    #[test]
    fn references_match_only_the_same_mutability() {
        let shared = Ty::Ref {
            mutable: false,
            inner: Box::new(Ty::Str),
        };
        let unique = Ty::Ref {
            mutable: true,
            inner: Box::new(Ty::Str),
        };
        let mut subst = Subst::new();
        assert_eq!(unify(&[], &shared, &shared, &mut subst), Ok(()));
        assert!(unify(&[], &shared, &unique, &mut subst).is_err());
    }

    #[test]
    fn two_types_of_the_same_name_in_different_modules_do_not_match() {
        let mut subst = Subst::new();
        assert!(matches!(
            unify(&[], &named(1, vec![]), &named(2, vec![]), &mut subst),
            Err(Mismatch::Shape { .. })
        ));
    }

    #[test]
    fn an_unresolved_type_matches_nothing_but_another_one() {
        let mut subst = Subst::new();
        assert_eq!(unify(&[], &Ty::Infer, &Ty::Infer, &mut subst), Ok(()));
        assert!(unify(&[], &Ty::Str, &Ty::Infer, &mut subst).is_err());
    }

    #[test]
    fn a_parameter_will_not_stand_for_an_unresolved_type() {
        // Nothing here ever works out what the `_` was, so an impl picked on the
        // strength of `T = _` would have been picked on nothing.
        let mut subst = Subst::new();
        assert!(matches!(
            unify(&["T".into()], &param("T"), &Ty::Infer, &mut subst),
            Err(Mismatch::Unresolved { .. })
        ));
        let mut subst = Subst::new();
        assert!(matches!(
            unify(
                &["T".into()],
                &named(1, vec![param("T")]),
                &named(1, vec![Ty::Infer]),
                &mut subst
            ),
            Err(Mismatch::Unresolved { .. })
        ));
    }
}
