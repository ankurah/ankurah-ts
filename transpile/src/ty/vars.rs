//! The unknowns of one function body, and the solver that binds them.
//!
//! A local written `let mut entities = Vec::new();` has no type until something
//! below it says so. The table mints a variable for that unknown, constraints
//! collected over the whole body are unified against each other through the
//! table, and what stays unbound at the end is reported rather than guessed.

use std::collections::HashMap;

use super::def::{InferId, TraitRef, Ty};
use super::unify::{unify_with, Mismatch, Unknowns};

/// Every inference variable of one body, and what each stands for.
///
/// A binding is made once and never revised, which is what makes the occurs
/// check enough to keep the table acyclic and the solve terminating.
#[derive(Debug, Default)]
pub struct InferTable {
    bound: Vec<Option<Ty>>,
    /// The variable standing at each written site. Two passes over one body ask
    /// about the same site, and the second must get the unknown the first
    /// bound rather than a new one.
    sites: HashMap<(usize, usize, usize), InferId>,
    /// Bindings in the order they were made, so a constraint that cannot be met
    /// leaves the table as it found it.
    journal: Vec<InferId>,
}

impl InferTable {
    pub fn new() -> InferTable {
        InferTable::default()
    }

    /// A new unknown, standing for nothing yet.
    pub fn fresh(&mut self) -> InferId {
        self.bound.push(None);
        InferId(self.bound.len() as u32 - 1)
    }

    /// The unknown standing at one written site — a source position and, where
    /// the site carries several, which one. Minted the first time and the same
    /// one after.
    pub fn at_site(&mut self, line: usize, col: usize, index: usize) -> InferId {
        match self.sites.get(&(line, col, index)) {
            Some(id) => *id,
            None => {
                let id = self.fresh();
                self.sites.insert((line, col, index), id);
                id
            }
        }
    }

    /// How many variables this body minted. The solve's round bound is read
    /// off it: no run may outlast one round per variable.
    pub fn len(&self) -> usize {
        self.bound.len()
    }

    /// How many variables stand for something. A solve round that leaves this
    /// unchanged has reached its fixed point.
    pub fn bound_count(&self) -> usize {
        self.bound.iter().filter(|b| b.is_some()).count()
    }

    /// What this variable stands for, one step, without following further.
    pub fn binding(&self, var: InferId) -> Option<&Ty> {
        self.bound.get(var.0 as usize).and_then(|b| b.as_ref())
    }

    /// The type with every bound variable replaced by what it stands for, all
    /// the way down. Variables nothing bound are left in place for the caller
    /// to report.
    pub fn resolve(&self, ty: &Ty) -> Ty {
        match ty {
            Ty::Var(id) => match self.binding(*id) {
                Some(bound) => self.resolve(bound),
                None => ty.clone(),
            },
            Ty::Named { id, args } => Ty::Named {
                id: *id,
                args: args.iter().map(|a| self.resolve(a)).collect(),
            },
            Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|e| self.resolve(e)).collect()),
            Ty::Ref { mutable, inner } => Ty::Ref {
                mutable: *mutable,
                inner: Box::new(self.resolve(inner)),
            },
            Ty::Slice(inner) => Ty::Slice(Box::new(self.resolve(inner))),
            Ty::Array { elem, len } => Ty::Array {
                elem: Box::new(self.resolve(elem)),
                len: len.clone(),
            },
            Ty::Dyn { traits } => Ty::Dyn {
                traits: traits.iter().map(|t| self.resolve_trait(t)).collect(),
            },
            Ty::ImplTrait { bounds } => Ty::ImplTrait {
                bounds: bounds.iter().map(|t| self.resolve_trait(t)).collect(),
            },
            Ty::Assoc { base, trait_, name } => Ty::Assoc {
                base: Box::new(self.resolve(base)),
                trait_: trait_.as_ref().map(|t| Box::new(self.resolve_trait(t))),
                name: name.clone(),
            },
            Ty::Param(_) | Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer => ty.clone(),
        }
    }

    fn resolve_trait(&self, trait_: &TraitRef) -> TraitRef {
        TraitRef {
            id: trait_.id,
            args: trait_.args.iter().map(|a| self.resolve(a)).collect(),
            bindings: trait_
                .bindings
                .iter()
                .map(|(n, t)| (n.clone(), self.resolve(t)))
                .collect(),
        }
    }

    /// Constrain two types to be the same, binding variables on either side.
    ///
    /// A shape the walk cannot reconcile is refused and binds nothing: a
    /// constraint with no solution must not leave behind the part of itself
    /// that did fit. The caller reports it at the site the constraint came
    /// from, naming both types.
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), Mismatch> {
        let made = self.journal.len();
        let answer = unify_with(&mut BodyVars { table: self }, a, b);
        if answer.is_err() {
            for var in self.journal.split_off(made) {
                self.bound[var.0 as usize] = None;
            }
        }
        answer
    }
}

/// The table as a unification walk sees it: an unknown on either side, bound
/// into the table itself.
struct BodyVars<'a> {
    table: &'a mut InferTable,
}

impl Unknowns for BodyVars<'_> {
    fn follow(&self, ty: &Ty) -> Option<Ty> {
        match ty {
            Ty::Var(id) => self.table.binding(*id).cloned(),
            _ => None,
        }
    }

    fn bind(&mut self, a: &Ty, b: &Ty) -> Option<Result<(), Mismatch>> {
        // Both sides arrive already followed, so a variable here stands for
        // nothing yet and binding it cannot overwrite an answer.
        match (a, b) {
            (Ty::Var(x), Ty::Var(y)) if x == y => Some(Ok(())),
            (Ty::Var(x), other) => Some(self.table.assign(*x, other)),
            (other, Ty::Var(y)) => Some(self.table.assign(*y, other)),
            // A bound and a type that meets it are not equal and not a
            // mismatch: `Arc<dyn TNode<CD>>` accepts an `Arc<MockNode>` by
            // coercion. The walk stops here rather than refusing, and what the
            // bound projects is constrained where the argument is read.
            (Ty::Dyn { .. } | Ty::ImplTrait { .. }, other)
            | (other, Ty::Dyn { .. } | Ty::ImplTrait { .. })
                if !matches!(other, Ty::Dyn { .. } | Ty::ImplTrait { .. }) =>
            {
                Some(Ok(()))
            }
            _ => None,
        }
    }
}

impl InferTable {
    fn assign(&mut self, var: InferId, ty: &Ty) -> Result<(), Mismatch> {
        if ty.mentions_var(var) {
            return Err(Mismatch::VarOccurs {
                var,
                ty: ty.clone(),
            });
        }
        self.bound[var.0 as usize] = Some(ty.clone());
        self.journal.push(var);
        Ok(())
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

    #[test]
    fn a_variable_binds_to_whatever_it_is_unified_with() {
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        assert_eq!(table.unify(&v, &Ty::Prim(Prim::U32)), Ok(()));
        assert_eq!(table.resolve(&v), Ty::Prim(Prim::U32));
    }

    #[test]
    fn a_variable_binds_from_either_side() {
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        assert_eq!(table.unify(&Ty::Str, &v), Ok(()));
        assert_eq!(table.resolve(&v), Ty::Str);
    }

    #[test]
    fn a_variable_inside_a_type_is_bound_by_the_position_it_stands_at() {
        // `Vec<?0>` against `Vec<u8>` is what a later `push` gives an empty
        // collection.
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        let pending = named(1, vec![v.clone()]);
        let known = named(1, vec![Ty::Prim(Prim::U8)]);
        assert_eq!(table.unify(&pending, &known), Ok(()));
        assert_eq!(table.resolve(&pending), known);
    }

    #[test]
    fn a_bound_and_a_type_that_meets_it_are_neither_equal_nor_a_mismatch() {
        // `Arc<dyn Trait>` accepts an `Arc<Concrete>` by coercion, so the walk
        // stops at the bound rather than refusing; what the bound projects is
        // constrained where the argument is read.
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        let bound = Ty::Dyn {
            traits: vec![crate::ty::TraitRef {
                id: TypeId(9),
                args: Vec::new(),
                bindings: Vec::new(),
            }],
        };
        assert_eq!(table.unify(&named(1, vec![bound]), &named(1, vec![Ty::Str])), Ok(()));
        assert_eq!(table.resolve(&v), v, "nothing was bound by a coercion");
    }

    #[test]
    fn a_binding_made_once_decides_every_later_comparison() {
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        assert_eq!(table.unify(&v, &Ty::Str), Ok(()));
        assert_eq!(table.unify(&v, &Ty::Str), Ok(()));
        assert!(matches!(
            table.unify(&v, &Ty::Prim(Prim::U8)),
            Err(Mismatch::Shape { .. })
        ));
        assert_eq!(table.resolve(&v), Ty::Str, "the first answer stands");
    }

    #[test]
    fn two_variables_unified_together_stand_for_one_type() {
        let mut table = InferTable::new();
        let a = Ty::Var(table.fresh());
        let b = Ty::Var(table.fresh());
        assert_eq!(table.unify(&a, &b), Ok(()));
        assert_eq!(table.unify(&b, &Ty::Prim(Prim::I64)), Ok(()));
        assert_eq!(table.resolve(&a), Ty::Prim(Prim::I64));
    }

    #[test]
    fn a_constraint_with_no_solution_binds_nothing() {
        // The first position fits and the second does not; leaving the first
        // behind would answer half a constraint that has no answer.
        let mut table = InferTable::new();
        let a = Ty::Var(table.fresh());
        let err = table.unify(
            &Ty::Tuple(vec![a.clone(), Ty::Str]),
            &Ty::Tuple(vec![Ty::Str, Ty::Unit]),
        );
        assert!(err.is_err(), "the second position cannot be met");
        assert_eq!(table.bound_count(), 0);
    }

    #[test]
    fn one_site_asked_twice_answers_with_the_same_unknown() {
        let mut table = InferTable::new();
        assert_eq!(table.at_site(4, 9, 0), table.at_site(4, 9, 0));
        assert_ne!(table.at_site(4, 9, 0), table.at_site(4, 9, 1));
        assert_ne!(table.at_site(4, 9, 0), table.at_site(5, 9, 0));
    }

    #[test]
    fn a_variable_unified_with_itself_binds_nothing() {
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        assert_eq!(table.unify(&v, &v), Ok(()));
        assert_eq!(table.bound_count(), 0);
    }

    #[test]
    fn a_variable_may_not_contain_itself() {
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        let recursive = named(1, vec![v.clone()]);
        assert!(matches!(
            table.unify(&v, &recursive),
            Err(Mismatch::VarOccurs { .. })
        ));
    }

    #[test]
    fn a_chain_of_variables_resolves_all_the_way_down() {
        let mut table = InferTable::new();
        let a = Ty::Var(table.fresh());
        let b = Ty::Var(table.fresh());
        let c = Ty::Var(table.fresh());
        assert_eq!(table.unify(&a, &b), Ok(()));
        assert_eq!(table.unify(&b, &c), Ok(()));
        assert_eq!(table.unify(&c, &named(7, vec![Ty::Str])), Ok(()));
        assert_eq!(table.resolve(&a), named(7, vec![Ty::Str]));
    }

    #[test]
    fn what_nothing_bound_is_reported_rather_than_filled_in() {
        let mut table = InferTable::new();
        let a = Ty::Var(table.fresh());
        let b = Ty::Var(table.fresh());
        let pair = Ty::Tuple(vec![a.clone(), b.clone(), a.clone()]);
        assert_eq!(table.unify(&a, &Ty::Str), Ok(()));
        assert!(table.resolve(&pair).mentions_any_var(), "one position is still open");
        assert_eq!(
            table.resolve(&pair),
            Ty::Tuple(vec![Ty::Str, b, Ty::Str]),
            "the solved positions still answer"
        );
    }

    #[test]
    fn a_written_underscore_is_not_a_variable() {
        // `_` is the source's hole; nothing here fills it, and treating it as a
        // variable would make the solver answer a question the source asked of
        // rustc.
        let mut table = InferTable::new();
        assert!(matches!(
            table.unify(&Ty::Infer, &Ty::Str),
            Err(Mismatch::Shape { .. })
        ));
    }

    #[test]
    fn a_shape_that_cannot_be_reconciled_is_refused_rather_than_chosen() {
        let mut table = InferTable::new();
        let v = Ty::Var(table.fresh());
        assert_eq!(table.unify(&named(1, vec![v]), &named(1, vec![Ty::Str])), Ok(()));
        let err = table.unify(&named(1, vec![Ty::Var(InferId(0))]), &named(1, vec![Ty::Unit]));
        assert!(
            matches!(err, Err(Mismatch::Shape { .. })),
            "neither side wins: {:?}",
            err
        );
    }
}
