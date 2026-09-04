//! Matching a written type against an impl's pattern.
//!
//! `impl<T> Signal for Arc<Inner<T>>` says nothing until something asks whether
//! a particular receiver is one of those. Unification answers that: it walks the
//! pattern and the concrete type together, binds each of the impl's own
//! parameters to whatever stands at its position, and refuses when the shapes
//! disagree.
//!
//! Only the pattern carries unknowns. A `Param` the impl declared is a variable;
//! every other `Param` — one the calling function declared, or `Self` inside a
//! trait — is rigid and matches only itself. That is what an impl actually
//! means: `impl<T> Foo for Vec<T>` applies to every `Vec`, while
//! `impl Foo for Vec<T>` written inside `impl<T>`'s body applies to that `T`.

use super::def::{TraitRef, Ty};
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
        }
    }
}

/// Match `concrete` against `pattern`, binding the pattern's variables.
///
/// `vars` names the parameters the pattern's owner declared. `subst` accumulates
/// the bindings and is left partly filled when the match fails, so a caller that
/// might retry hands in a fresh one.
pub fn unify(vars: &[String], pattern: &Ty, concrete: &Ty, subst: &mut Subst) -> Result<(), Mismatch> {
    match (pattern, concrete) {
        (Ty::Param(name), _) if vars.iter().any(|v| v == name) => bind(name, concrete, subst),

        // A rigid parameter stands for one specific type, so only that same
        // parameter matches it.
        (Ty::Param(a), Ty::Param(b)) if a == b => Ok(()),

        (Ty::Named { id: a, args: xs }, Ty::Named { id: b, args: ys })
            if a == b && xs.len() == ys.len() =>
        {
            for (x, y) in xs.iter().zip(ys) {
                unify(vars, x, y, subst)?;
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
        ) if a == b => unify(vars, x, y, subst),

        (Ty::Tuple(xs), Ty::Tuple(ys)) if xs.len() == ys.len() => {
            for (x, y) in xs.iter().zip(ys) {
                unify(vars, x, y, subst)?;
            }
            Ok(())
        }

        (Ty::Slice(x), Ty::Slice(y)) => unify(vars, x, y, subst),

        (Ty::Array { elem: x, len: n }, Ty::Array { elem: y, len: m }) if n == m => {
            unify(vars, x, y, subst)
        }

        (Ty::Dyn { traits: xs }, Ty::Dyn { traits: ys }) if xs.len() == ys.len() => {
            for (x, y) in xs.iter().zip(ys) {
                unify_trait(vars, x, y, subst)?;
            }
            Ok(())
        }

        (Ty::ImplTrait { bounds: xs }, Ty::ImplTrait { bounds: ys }) if xs.len() == ys.len() => {
            for (x, y) in xs.iter().zip(ys) {
                unify_trait(vars, x, y, subst)?;
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
                (Some(x), Some(y)) => unify_trait(vars, x, y, subst)?,
                (None, None) => {}
                _ => {
                    return Err(Mismatch::Shape {
                        pattern: pattern.clone(),
                        concrete: concrete.clone(),
                    })
                }
            }
            unify(vars, xb, yb, subst)
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

fn unify_trait(
    vars: &[String],
    pattern: &TraitRef,
    concrete: &TraitRef,
    subst: &mut Subst,
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
        unify(vars, x, y, subst)?;
    }
    for (name, x) in &pattern.bindings {
        match concrete.bindings.iter().find(|(n, _)| n == name) {
            Some((_, y)) => unify(vars, x, y, subst)?,
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

fn bind(param: &str, ty: &Ty, subst: &mut Subst) -> Result<(), Mismatch> {
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
    // `_` is a hole the expected-type step fills (spec 4.6). Binding a parameter
    // to one would let an impl be selected on a receiver nobody has typed yet.
    if ty.mentions_infer() {
        return Err(Mismatch::Unresolved {
            param: param.to_string(),
        });
    }
    subst.insert(param.to_string(), ty.clone());
    Ok(())
}

impl Ty {
    /// Does this type mention the parameter by that name anywhere inside it?
    pub fn mentions_param(&self, name: &str) -> bool {
        match self {
            Ty::Param(p) => p == name,
            Ty::Named { args, .. } | Ty::Tuple(args) => {
                args.iter().any(|a| a.mentions_param(name))
            }
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.mentions_param(name)
            }
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
                traits.iter().any(|t| t.mentions_param(name))
            }
            Ty::Assoc { base, trait_, .. } => {
                base.mentions_param(name)
                    || trait_.as_ref().is_some_and(|t| t.mentions_param(name))
            }
            Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer => false,
        }
    }
}

impl Ty {
    /// Is there a `_` anywhere inside this type?
    pub fn mentions_infer(&self) -> bool {
        match self {
            Ty::Infer => true,
            Ty::Named { args, .. } | Ty::Tuple(args) => args.iter().any(|a| a.mentions_infer()),
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.mentions_infer()
            }
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
                traits.iter().any(|t| t.mentions_infer())
            }
            Ty::Assoc { base, .. } => base.mentions_infer(),
            Ty::Param(_) | Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never => false,
        }
    }
}

impl TraitRef {
    pub fn mentions_infer(&self) -> bool {
        self.args.iter().any(|a| a.mentions_infer())
            || self.bindings.iter().any(|(_, t)| t.mentions_infer())
    }

    pub fn mentions_param(&self, name: &str) -> bool {
        self.args.iter().any(|a| a.mentions_param(name))
            || self.bindings.iter().any(|(_, t)| t.mentions_param(name))
    }
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
