//! How a constraint lines up the references on its two sides.

use crate::testing::Fixture;
use crate::ty::Ty;

fn at() -> proc_macro2::Span {
    proc_macro2::Span::call_site()
}

/// A context with the constraint walk running, which is the only walk that
/// binds.
fn solving<'a>(cx: crate::infer::TypeContext<'a>) -> crate::infer::TypeContext<'a> {
    cx.vars.borrow_mut().set_solving(true);
    cx
}

fn borrow(inner: Ty) -> Ty {
    Ty::Ref {
        mutable: false,
        inner: Box::new(inner),
    }
}

#[test]
fn a_by_value_parameter_facing_a_borrow_binds_to_the_borrow() {
    // What `picked.push(t)` says about `picked` when `t` is a `&Thing`: the
    // collection holds borrows, and a collection of borrows releases nothing.
    let c = Fixture::build(&[("lib.rs", "pub struct Thing;")]);
    let cx = solving(c.context("lib.rs", None));
    let thing = c.named("lib.rs", "Thing", vec![]);

    let element = cx.var_at(at(), 0);
    let collection = c.system("std::vec::Vec", vec![element.clone()]);
    cx.constrain_here(at(), &element, &borrow(thing.clone()));

    assert_eq!(cx.solved(&element), borrow(thing.clone()));
    assert_eq!(
        cx.solved(&collection),
        c.system("std::vec::Vec", vec![borrow(thing)])
    );
}

#[test]
fn two_borrows_of_the_same_mutability_meet_through_what_they_refer_to() {
    let c = Fixture::build(&[("lib.rs", "pub struct Thing;")]);
    let cx = solving(c.context("lib.rs", None));
    let thing = c.named("lib.rs", "Thing", vec![]);

    let unknown = cx.var_at(at(), 0);
    cx.constrain_here(
        at(),
        &borrow(c.system("std::vec::Vec", vec![unknown.clone()])),
        &borrow(c.system("std::vec::Vec", vec![thing.clone()])),
    );

    assert_eq!(cx.solved(&unknown), thing);
}

#[test]
fn a_borrow_the_engine_reads_the_value_through_is_not_a_disagreement() {
    // `Arc::downgrade(self)` declares `&Arc<T>` and the engine reads `self` as
    // the `Arc` itself; the declared `&` is how it reads that value, so `T`
    // still binds and nothing is reported.
    let c = Fixture::build(&[("lib.rs", "use std::sync::Arc;\npub struct Thing;")]);
    let cx = solving(c.context("lib.rs", None));
    let thing = c.named("lib.rs", "Thing", vec![]);

    let unknown = cx.var_at(at(), 0);
    cx.constrain_here(
        at(),
        &borrow(c.system("std::sync::Arc", vec![unknown.clone()])),
        &c.system("std::sync::Arc", vec![thing.clone()]),
    );

    assert_eq!(cx.solved(&unknown), thing);
    assert!(c.messages().is_empty(), "{:?}", c.messages());
}

#[test]
fn the_walk_that_writes_a_body_says_so_without_binding() {
    // A binding made while the body is written reaches the statements below it
    // and not the ones above, which is one local with two answers.
    let c = Fixture::build(&[("lib.rs", "pub struct Thing;")]);
    let cx = c.context("lib.rs", None);
    let thing = c.named("lib.rs", "Thing", vec![]);

    let unknown = cx.var_at(at(), 0);
    cx.constrain_here(at(), &unknown, &thing);
    assert_eq!(cx.solved(&unknown), unknown, "the writing walk bound an unknown");

    cx.constrain_here(at(), &Ty::Str, &c.system("std::vec::Vec", vec![unknown]));
    assert_eq!(
        c.messages().len(),
        1,
        "two types that cannot meet are still said: {:?}",
        c.messages()
    );
}
