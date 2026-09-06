//! What a projection the impl table cannot settle is still good for.
//!
//! Spec 4.4a: `<I as IntoIterator>::IntoIter` in a body that never learns which
//! iterator it is has no impl to select, and Rust still lets `.next()` be
//! called on it — because `IntoIterator` DECLARES `type IntoIter: Iterator<Item
//! = Self::Item>`. These ask that the declaration is kept, instantiated for the
//! projection, and dispatched through.

use crate::testing::Fixture;
use crate::ty::{Prim, TraitRef, Ty};

/// `I: IntoIterator<Item = u32>` in scope, and the projection that stands for
/// what `I::into_iter` hands back.
fn a_bounded_parameter() -> (Fixture, Vec<(String, TraitRef)>) {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let into_iterator = c.system_id("std::iter::IntoIterator");
    let bounds = vec![(
        "I".to_string(),
        TraitRef {
            id: into_iterator,
            args: Vec::new(),
            bindings: vec![("Item".to_string(), Ty::Prim(Prim::U32))],
        },
    )];
    (c, bounds)
}

fn into_iter_of(c: &Fixture) -> Ty {
    Ty::Assoc {
        base: Box::new(Ty::Param("I".to_string())),
        trait_: Some(Box::new(TraitRef {
            id: c.system_id("std::iter::IntoIterator"),
            args: Vec::new(),
            bindings: Vec::new(),
        })),
        name: "IntoIter".to_string(),
    }
}

#[test]
fn a_projection_carries_the_bounds_its_declaration_gives_it() {
    let (c, bounds) = a_bounded_parameter();
    let probe = c.probe("lib.rs").with_bounds(&bounds);
    // `next` is `Iterator`'s, and nothing but the declared bound says the
    // projection is one: no impl of `IntoIterator` for `I` exists to select.
    let found = probe
        .resolve_method(&into_iter_of(&c), "next")
        .expect("the declared bound answers");
    // `Iterator::next` hands back `Option<Self::Item>`, and `Self::Item` is the
    // `u32` the bound at the use site bound it to.
    assert_eq!(found.ret, c.system("std::option::Option", vec![Ty::Prim(Prim::U32)]));
}

#[test]
fn a_projection_with_no_declared_bound_answers_no_method() {
    let (c, bounds) = a_bounded_parameter();
    let probe = c.probe("lib.rs").with_bounds(&bounds);
    // `Item` is declared with no bounds at all, so the projection standing for
    // it is a type nothing can be called on — which is the truth about it.
    let item = Ty::Assoc {
        base: Box::new(Ty::Param("I".to_string())),
        trait_: Some(Box::new(TraitRef {
            id: c.system_id("std::iter::IntoIterator"),
            args: Vec::new(),
            bindings: Vec::new(),
        })),
        name: "Item".to_string(),
    };
    assert!(probe.resolve_method(&item, "next").is_err());
}

#[test]
fn the_port_writes_a_declared_iterator_as_a_sequence() {
    let (c, bounds) = a_bounded_parameter();
    let probe = c.probe("lib.rs").with_bounds(&bounds);
    // The port writes `values.into_iter()` as `[...values]`, so a call on what
    // it hands back is a call on an array.
    assert_eq!(
        probe.written_as_sequence(&into_iter_of(&c)),
        Some(Ty::Slice(Box::new(Ty::Prim(Prim::U32))))
    );
    // A type that is already a sequence answers for itself, and so does one
    // that is not a projection at all.
    assert_eq!(
        probe.written_as_sequence(&c.system("std::vec::Vec", vec![Ty::Prim(Prim::U32)])),
        None
    );
    assert_eq!(probe.written_as_sequence(&Ty::Prim(Prim::U32)), None);
}

#[test]
fn a_bound_the_declaration_wrote_reaches_through_a_supertrait() {
    // `type Parent: TClock<Id = Self::Id>` on a trait whose associated type is
    // named through a bound rather than an impl — core's `retrieval.rs` shape,
    // where `G::Event: TEvent` and `TEvent::Parent: TClock` between them make
    // `event.parent().members()` a call the engine can write.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait TClock { fn members(&self) -> u32; }\n\
         pub trait TEvent { type Parent: TClock; fn parent(&self) -> Self::Parent; }\n\
         pub struct S;",
    )]);
    let tevent = c.reg.module_type(c.module("lib.rs"), "TEvent").expect("declared");
    let bounds = vec![(
        "E".to_string(),
        TraitRef { id: tevent, args: Vec::new(), bindings: Vec::new() },
    )];
    let probe = c.probe("lib.rs").with_bounds(&bounds);
    let parent = probe
        .resolve_method(&Ty::Param("E".to_string()), "parent")
        .expect("the bound answers `parent`");
    // Its return type is the projection, and the projection is a `TClock` by
    // declaration — so `members` resolves on it with no impl in sight.
    let members = probe
        .resolve_method(&parent.ret, "members")
        .expect("the associated type's own declared bound answers `members`");
    assert_eq!(members.ret, Ty::Prim(Prim::U32));
}
