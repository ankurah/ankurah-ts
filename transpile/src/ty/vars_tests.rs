//! What the table promises: a binding made once, a constraint with no solution
//! binding nothing, and a contradiction that outlives the walk that met it.

use super::vars::InferTable;
use super::def::InferId;
use super::unify::Mismatch;
use super::vars::VarKind;
use crate::ty::{Prim, Ty, TypeId};

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

#[test]
fn a_poisoned_variable_stands_for_nothing() {
    // The binding that came first is not an answer: a constraint with no
    // solution touched it, so it resolves as the unknown it is.
    let mut table = InferTable::new();
    let id = table.fresh();
    let v = Ty::Var(id);
    assert_eq!(table.unify(&v, &Ty::Str), Ok(()));
    table.poison_ids([id]);
    assert_eq!(table.resolve(&v), v);
}

#[test]
fn a_poisoned_variable_meets_anything_and_holds_nothing() {
    // One constraint with no solution is the report; every constraint after it
    // would say the same failure again in another position.
    let mut table = InferTable::new();
    let id = table.fresh();
    let v = Ty::Var(id);
    table.poison_ids([id]);
    assert_eq!(table.unify(&v, &Ty::Prim(Prim::U8)), Ok(()));
    assert_eq!(table.resolve(&v), v);
}

#[test]
fn a_failed_walk_names_every_unknown_it_went_through() {
    // What a constraint stands on is what the walk followed, on both sides.
    // Poisoning reads this rather than trying to recover a variable from types
    // the solve has already replaced by what they stand for.
    let mut table = InferTable::new();
    let a = table.fresh();
    let b = table.fresh();
    assert_eq!(table.unify(&Ty::Var(a), &Ty::Str), Ok(()));
    assert_eq!(table.unify(&Ty::Var(b), &Ty::Prim(Prim::U8)), Ok(()));
    assert!(table.unify(&Ty::Var(a), &Ty::Var(b)).is_err());
    assert_eq!(table.touched(), vec![a, b]);
}

#[test]
fn a_failed_walk_names_an_unknown_inside_the_shape() {
    // `Vec<?0>` against `Vec<usize>` stands on `?0`, which is the whole of what
    // the collection's element disagreeing means.
    let mut table = InferTable::new();
    let elem = table.fresh();
    assert_eq!(table.unify(&Ty::Var(elem), &Ty::Str), Ok(()));
    assert!(table
        .unify(
            &named(1, vec![Ty::Var(elem)]),
            &named(1, vec![Ty::Prim(Prim::U8)])
        )
        .is_err());
    assert_eq!(table.touched(), vec![elem]);
}

#[test]
fn what_the_solve_decided_only_ever_grows() {
    // The fixed point stops when a round decides nothing new, so poisoning has
    // to count as a decision or the walk would never come to rest.
    let mut table = InferTable::new();
    let id = table.fresh();
    let before = table.bound_count();
    table.poison_ids([id]);
    assert!(table.bound_count() > before);
}

#[test]
fn a_restricted_variable_binds_only_what_its_kind_admits() {
    let mut table = InferTable::new();
    let n = Ty::Var(table.kinded_at_site(1, 1, 0, VarKind::Integral));
    assert!(matches!(table.unify(&n, &Ty::Unit), Err(Mismatch::Kind { .. })));
    assert_eq!(table.unify(&n, &Ty::Prim(Prim::Usize)), Ok(()));
    assert_eq!(table.resolve(&n), Ty::Prim(Prim::Usize));
}

#[test]
fn a_restricted_variable_nothing_bound_takes_rusts_own_fallback() {
    let mut table = InferTable::new();
    let n = Ty::Var(table.kinded_at_site(1, 1, 0, VarKind::Integral));
    let x = Ty::Var(table.kinded_at_site(2, 1, 0, VarKind::Float));
    table.settle_kinds();
    assert_eq!(table.resolve(&n), Ty::Prim(Prim::I32));
    assert_eq!(table.resolve(&x), Ty::Prim(Prim::F64));
}

#[test]
fn a_restriction_travels_to_the_variable_it_meets() {
    // `let mut n = 0; let m = n;` makes one question of two, and the answer to
    // both is still only a number.
    let mut table = InferTable::new();
    let n = Ty::Var(table.kinded_at_site(1, 1, 0, VarKind::Integral));
    let m = Ty::Var(table.fresh());
    assert_eq!(table.unify(&n, &m), Ok(()));
    assert!(matches!(table.unify(&m, &Ty::Str), Err(Mismatch::Kind { .. })));
}

#[test]
fn a_variable_may_not_contain_itself_through_another() {
    // `a = b` then `b = Vec<a>` is the same infinite type one step apart, and
    // taking it would make resolving `a` recurse forever.
    let mut table = InferTable::new();
    let a = Ty::Var(table.fresh());
    let b = Ty::Var(table.fresh());
    assert_eq!(table.unify(&a, &b), Ok(()));
    assert!(matches!(
        table.unify(&b, &named(1, vec![a.clone()])),
        Err(Mismatch::VarOccurs { .. })
    ));
    assert_eq!(table.resolve(&a), b, "the cycle bound nothing; `a` still stands for `b`");
}
