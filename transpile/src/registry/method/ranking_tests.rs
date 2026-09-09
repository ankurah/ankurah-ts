//! Which candidate wins when two answer to one name.
//!
//! A candidate that applies outright beats one that applies only IF something
//! nobody has decided holds — at every depth of the deref chain, because
//! picking the conditional one answers a question the engine never settled.

use super::{Callee, DerefKind, MethodError};
use crate::testing::Fixture;
use crate::ty::{Prim, Ty};

#[test]
fn a_candidate_with_an_open_obligation_loses_to_one_a_deref_reaches() {
    // `impl<T: Red> Ext for Wrap<T>` applies only where `T` is `Red`, and
    // nothing here has settled what `T` is. Picking it answers a question the
    // engine never closed, so `Inner::go`, one `Deref` away and conditional on
    // nothing, is the answer.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Red { fn red(&self) -> u32; }\n\
         pub struct Inner;\n\
         impl Inner { pub fn go(&self) -> u32 { 20 } }\n\
         pub struct Wrap<T> { pub held: Vec<T>, pub inner: Inner }\n\
         impl<T> std::ops::Deref for Wrap<T> {\n    \
             type Target = Inner;\n    \
             fn deref(&self) -> &Inner { &self.inner }\n\
         }\n\
         pub trait Ext { fn go(&self) -> u32; }\n\
         impl<T: Red> Ext for Wrap<T> { fn go(&self) -> u32 { 99 } }",
    )]);
    let wrap = c.named("lib.rs", "Wrap", vec![Ty::Param("T".to_string())]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&wrap, "go")
        .expect("reached through Deref");
    assert!(matches!(found.callee, Callee::Inherent(..)), "{:?}", found.callee);
    assert_eq!(found.steps.len(), 1);
    assert!(matches!(found.steps[0].kind, DerefKind::Overloaded(_)));
    assert_eq!(found.ret, Ty::Prim(Prim::U32));
    assert!(found.obligations.is_empty(), "{:?}", found.obligations);
}

#[test]
fn a_candidate_with_an_open_obligation_still_answers_where_nothing_else_does() {
    // The rule is a ranking, not a filter: where the conditional impl is the
    // only candidate, it is still the answer and the obligation is still said.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Red { fn red(&self) -> u32; }\n\
         pub struct Wrap<T> { pub held: Vec<T> }\n\
         pub trait Ext { fn go(&self) -> u32; }\n\
         impl<T: Red> Ext for Wrap<T> { fn go(&self) -> u32 { 99 } }",
    )]);
    let wrap = c.named("lib.rs", "Wrap", vec![Ty::Param("T".to_string())]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&wrap, "go")
        .expect("the only candidate answers");
    assert!(!found.obligations.is_empty(), "the bound is still open");
}

#[test]
fn two_impls_a_where_clause_tells_apart_are_a_tie_while_the_subject_is_open() {
    // Neither bound can be checked until something says what `T` is, so both
    // impls are still candidates and the call is a tie — the answer contract
    // 4.1 promises, and not "no such method".
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait First { fn first(&self) -> u32; }\n\
         pub trait Second { fn second(&self) -> u32; }\n\
         pub struct Foo<T> { pub held: Vec<T> }\n\
         pub trait Pick { fn pick(&self) -> u32; }\n\
         impl<T: First> Pick for Foo<T> { fn pick(&self) -> u32 { 1 } }\n\
         impl<T: Second> Pick for Foo<T> { fn pick(&self) -> u32 { 2 } }",
    )]);
    let foo = c.named("lib.rs", "Foo", vec![Ty::Param("T".to_string())]);
    let err = c.probe("lib.rs").resolve_method(&foo, "pick").unwrap_err();
    assert!(matches!(err, MethodError::Ambiguous { .. }), "{:?}", err);
}
