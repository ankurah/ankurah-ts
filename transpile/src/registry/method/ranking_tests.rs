//! Which candidate wins when two answer to one name.
//!
//! A candidate that applies outright beats one that applies only IF something
//! nobody has decided holds — at every depth of the deref chain, because
//! picking the conditional one answers a question the engine never settled.

use super::{Callee, DerefKind, MethodError, Undecided};
use crate::testing::Fixture;
use crate::ty::{InferId, Prim, Ty};

/// `Inner::go` one `Deref` away, and an extension impl at depth 0 that applies
/// only where the wrapper's element is `Red`.
const DEREF_AND_A_CONDITIONAL_EXT: &str = "pub trait Red { fn red(&self) -> u32; }\n\
     pub struct Inner;\n\
     impl Inner { pub fn go(&self) -> u32 { 20 } }\n\
     pub struct Wrap<T> { pub held: Vec<T>, pub inner: Inner }\n\
     impl<T> std::ops::Deref for Wrap<T> {\n    \
         type Target = Inner;\n    \
         fn deref(&self) -> &Inner { &self.inner }\n\
     }\n\
     pub trait Ext { fn go(&self) -> u32; }\n\
     impl<T: Red> Ext for Wrap<T> { fn go(&self) -> u32 { 99 } }";

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

#[test]
fn an_unsettled_subject_stops_the_walk_at_its_own_depth() {
    // `T` here is an unknown the solve has not bound, not a parameter nothing
    // can bind. Descending past it picks `Inner::go` where Rust calls
    // `Ext::go`, so the shallow candidate stands and its bound is said.
    let c = Fixture::build(&[("lib.rs", DEREF_AND_A_CONDITIONAL_EXT)]);
    let wrap = c.named("lib.rs", "Wrap", vec![Ty::Var(InferId(0))]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&wrap, "go")
        .expect("the shallow candidate stands");
    assert!(found.steps.is_empty(), "{:?}", found.steps);
    assert_eq!(found.obligations.len(), 1);
    assert_eq!(found.obligations[0].reason, Undecided::Unsettled);
}

#[test]
fn a_call_the_deref_chain_could_answer_differently_is_contested() {
    // A second answer further down means the call's RESULT is decided by the
    // bound, so the type it produces is not a fact the body may read yet.
    let c = Fixture::build(&[("lib.rs", DEREF_AND_A_CONDITIONAL_EXT)]);
    let wrap = c.named("lib.rs", "Wrap", vec![Ty::Var(InferId(0))]);
    let found = c.probe("lib.rs").resolve_method(&wrap, "go").unwrap();
    assert!(found.contested);
}

#[test]
fn a_bound_that_concretely_fails_still_loses_to_the_deref() {
    // The narrowing is by REASON. `B` is a type, and it is not `Red`: that is a
    // fact about the program, so the extension impl is out and the method one
    // `Deref` away is the answer, with nothing left open.
    let c = Fixture::build(&[(
        "lib.rs",
        &format!("{}\npub struct B;", DEREF_AND_A_CONDITIONAL_EXT),
    )]);
    let b = c.named("lib.rs", "B", vec![]);
    let wrap = c.named("lib.rs", "Wrap", vec![b]);
    let found = c.probe("lib.rs").resolve_method(&wrap, "go").unwrap();
    assert_eq!(found.steps.len(), 1);
    assert!(found.obligations.is_empty(), "{:?}", found.obligations);
    assert!(!found.contested);
}
