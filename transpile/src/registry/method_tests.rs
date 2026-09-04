//! Method resolution's executable specification.
//!
//! Each test writes a few lines of Rust, builds the impl table from them, and
//! asks which function a call lands on — the receivers it walked through, the
//! borrow it took, and the type it hands back. The negative cases are here for
//! the same reason as the positive ones: refusing is an answer the translator
//! depends on.

use super::method::{Callee, DerefKind, Undecided};
use super::method::MethodError;
use super::method::AutoRef;
use crate::testing::Fixture;
use crate::ty::{Prim, Ty};

const ARC: &str = "std::sync::Arc";
const MUTEX: &str = "std::sync::Mutex";
const VEC: &str = "std::vec::Vec";
const HASHMAP: &str = "std::collections::HashMap";

// ── The deref chain ───────────────────────────────────────────────────

#[test]
fn a_call_reaches_through_two_guard_layers() {
    // `Arc<Mutex<Vec<T>>>::len` is `Vec::len`, reached by dereferencing the
    // `Arc`, calling `lock`, and dereferencing the guard. Each hop that is
    // written in TypeScript contributes its accessor, in order.
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let held = c.system(
        ARC,
        vec![c.system(
            MUTEX,
            vec![c.system(VEC, vec![Ty::Prim(Prim::U8)])],
        )],
    );
    let probe = c.probe("lib.rs");

    // The `Arc` is reached through first, then `lock` answers on the `Mutex`.
    let lock = probe.resolve_method(&held, "lock").expect("Mutex::lock");
    assert_eq!(lock.accessors(), vec!["value"]);
    assert_eq!(lock.steps.len(), 1);
    assert_eq!(lock.autoref, AutoRef::Shared);

    // `lock()` hands back a `LockResult`, as Rust's does, and the `unwrap` the
    // source writes is `Result::unwrap` on it. Nothing is reached through: a
    // `Result` is not a wrapper the port writes an accessor for.
    let guard = probe
        .resolve_method(&lock.ret, "unwrap")
        .expect("Result::unwrap on the LockResult");
    assert!(guard.accessors().is_empty());
    assert_eq!(
        guard.ret,
        c.system("std::sync::MutexGuard", vec![c.system(VEC, vec![Ty::Prim(Prim::U8)])])
    );

    // And the guard it hands back reaches the `Vec` inside it.
    let len = probe.resolve_method(&guard.ret, "len").expect("Vec::len");
    assert_eq!(len.accessors(), vec!["value"]);
    assert_eq!(len.ret, Ty::Prim(Prim::Usize));
}

#[test]
fn a_written_deref_impl_is_an_ordinary_step() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Inner { pub v: u8 }\n\
         pub struct Wrapper(Inner);\n\
         impl Inner { pub fn peek(&self) -> u8 { 0 } }\n\
         impl std::ops::Deref for Wrapper { type Target = Inner; fn deref(&self) -> &Inner { &self.0 } }",
    )]);
    let wrapper = c.named("lib.rs", "Wrapper", vec![]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&wrapper, "peek")
        .expect("reached through the crate's own Deref");
    assert_eq!(found.steps.len(), 1);
    assert!(matches!(found.steps[0].kind, DerefKind::Overloaded(_)));
    assert_eq!(found.ret, Ty::Prim(Prim::U8));
    // Rust inserts a `deref()` call here, and the emitted class carries one, so
    // the TypeScript writes it too. Without it the field behind the wrapper is
    // read straight off the wrapper, which cannot run.
    assert_eq!(found.accessors(), vec!["deref()".to_string()]);
}

#[test]
fn a_vec_reaches_the_slices_methods_and_an_array_is_unsized_first() {
    // `split_last` is declared once, on `[T]`, by the std surface; the fixture
    // supplies nothing, because a second `impl [u8]` here would be the duplicate
    // definition rustc calls E0592.
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let probe = c.probe("lib.rs");

    let vec = c.system(VEC, vec![Ty::Prim(Prim::U8)]);
    let found = probe
        .resolve_method(&vec, "split_last")
        .expect("a Vec dereferences to its slice");
    assert_eq!(found.steps.len(), 1);
    assert_eq!(found.steps[0].to, Ty::Slice(Box::new(Ty::Prim(Prim::U8))));
    assert!(found.accessors().is_empty(), "the hop is invisible in JS");

    let array = Ty::Array {
        elem: Box::new(Ty::Prim(Prim::U8)),
        len: crate::ty::ArrayLen::Lit(4),
    };
    let found = probe
        .resolve_method(&array, "split_last")
        .expect("an array is unsized to its slice");
    assert_eq!(found.steps.len(), 1);
    assert_eq!(found.steps[0].kind, DerefKind::Unsize);
}

#[test]
fn a_string_dereferences_to_str() {
    // `chars` comes from the surface's own `impl str`, for the same reason.
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let string = c.system("std::string::String", vec![]);
    let borrowed = Ty::Ref {
        mutable: false,
        inner: Box::new(string),
    };
    let found = c
        .probe("lib.rs")
        .resolve_method(&borrowed, "chars")
        .expect("&String to String to str");
    assert_eq!(found.steps.len(), 2, "the borrow, then the Deref");
    assert_eq!(found.steps[0].kind, DerefKind::Builtin);
    assert_eq!(found.steps[1].to, Ty::Str);
    assert_eq!(found.autoref, AutoRef::Shared, "and then `&str`");
}

// ── Which borrow the call takes ───────────────────────────────────────

#[test]
fn the_borrow_is_the_one_the_method_declares() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         impl S {\n\
           pub fn consume(self) -> u8 { 0 }\n\
           pub fn read(&self) -> u16 { 0 }\n\
           pub fn write(&mut self) -> u32 { 0 }\n\
         }",
    )]);
    let s = c.named("lib.rs", "S", vec![]);
    let probe = c.probe("lib.rs");
    for (method, autoref) in [
        ("consume", AutoRef::None),
        ("read", AutoRef::Shared),
        ("write", AutoRef::Mut),
    ] {
        let found = probe.resolve_method(&s, method).expect(method);
        assert_eq!(found.autoref, autoref, "{}", method);
        assert!(found.steps.is_empty(), "{}", method);
    }
}

// ── Which impl answers ────────────────────────────────────────────────

#[test]
fn an_inherent_method_wins_over_a_trait_one_of_the_same_name() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         pub trait Named { fn name(&self) -> u16; }\n\
         impl S { pub fn name(&self) -> u8 { 0 } }\n\
         impl Named for S { fn name(&self) -> u16 { 0 } }",
    )]);
    let s = c.named("lib.rs", "S", vec![]);
    let found = c.probe("lib.rs").resolve_method(&s, "name").expect("resolves");
    assert!(matches!(found.callee, Callee::Inherent(..)), "{:?}", found.callee);
    assert_eq!(found.ret, Ty::Prim(Prim::U8));
}

/// Two traits offering one name at the same receiver is what Rust calls E0034,
/// checked against rustc: `impl Direct for Conc` and `impl<T> Wide for T` both
/// answer `c.tag()` on a `&Conc`, and rustc refuses to choose. The engine used
/// to take the blanket silently, because it compared the borrow the method
/// wants against the borrow being tried rather than comparing receiver types:
/// `Direct::tag(&self)` accepts a `&Conc` with no borrow added, so it competes
/// at the very first step, one step before the engine looked for it.
#[test]
fn two_traits_offering_one_name_on_a_reference_is_an_ambiguity() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Conc;\n\
         pub trait Direct { fn tag(&self) -> u8; }\n\
         pub trait Wide { fn tag(&self) -> u16; }\n\
         impl Direct for Conc { fn tag(&self) -> u8 { 0 } }\n\
         impl<T> Wide for T { fn tag(&self) -> u16 { 0 } }",
    )]);
    let by_reference = Ty::Ref {
        mutable: false,
        inner: Box::new(c.named("lib.rs", "Conc", vec![])),
    };
    let err = c
        .probe("lib.rs")
        .resolve_method(&by_reference, "tag")
        .expect_err("rustc reports E0034 here, and so does the engine");
    let message = err.describe(&c.reg, "tag");
    assert!(message.contains("ambiguous"), "{}", message);
    assert!(message.contains("Direct") && message.contains("Wide"), "{}", message);
}

/// The same two traits with only one of them in scope: the tie-break settles it
/// rather than the tier order, and the answer is the one Rust would give.
#[test]
fn a_trait_out_of_scope_loses_the_tie_break() {
    let c = Fixture::build(&[
        (
            "lib.rs",
            "pub mod hidden;\npub mod here;\npub struct Conc;",
        ),
        (
            "hidden.rs",
            "pub trait Wide { fn tag(&self) -> u16; }\n\
             impl<T> Wide for T { fn tag(&self) -> u16 { 0 } }",
        ),
        (
            "here.rs",
            "use crate::Conc;\n\
             pub trait Direct { fn tag(&self) -> u8; }\n\
             impl Direct for Conc { fn tag(&self) -> u8 { 0 } }",
        ),
    ]);
    let by_reference = Ty::Ref {
        mutable: false,
        inner: Box::new(c.named("lib.rs", "Conc", vec![])),
    };
    let found = c
        .probe("here.rs")
        .resolve_method(&by_reference, "tag")
        .expect("only `Direct` is nameable here");
    assert_eq!(found.ret, Ty::Prim(Prim::U8));
}

#[test]
fn a_blanket_impl_answers_when_nothing_else_does() {
    // `impl<T: Display> ToString for T`, the shape behind `.to_string()` on a
    // primitive. The bound here is on a trait nothing declares, so it cannot be
    // decided and travels with the answer instead of being assumed. (`Display`
    // itself is declared now, and a bound on it is decided.)
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Show { fn show(&self) -> u8; }\n\
         impl<T: nowhere::Shown> Show for T { fn show(&self) -> u8 { 0 } }",
    )]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&Ty::Prim(Prim::I16), "show")
        .expect("the blanket impl applies");
    assert!(matches!(found.callee, Callee::Blanket(..)), "{:?}", found.callee);
    assert_eq!(found.obligations.len(), 1);
    assert_eq!(found.obligations[0].reason, Undecided::NoDeclaration);
    assert_eq!(found.obligations[0].subject, Ty::Prim(Prim::I16));
}

#[test]
fn a_blanket_impl_whose_bound_is_known_to_fail_is_not_a_candidate() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Yes;\npub struct No;\n\
         pub trait Marker {}\n\
         impl Marker for Yes {}\n\
         pub trait Show { fn show(&self) -> u8; }\n\
         impl<T: Marker> Show for T { fn show(&self) -> u8 { 0 } }",
    )]);
    let probe = c.probe("lib.rs");
    assert!(
        probe
            .resolve_method(&c.named("lib.rs", "Yes", vec![]), "show")
            .is_ok(),
        "the bound holds"
    );
    let refused = probe.resolve_method(&c.named("lib.rs", "No", vec![]), "show");
    assert!(
        matches!(refused, Err(MethodError::NotFound { .. })),
        "the bound does not hold, so the impl is not a candidate"
    );
}

#[test]
fn an_impl_on_a_wrapped_type_is_not_an_impl_on_the_wrapper() {
    // `impl Signal for Arc<Inner<T>>` is an impl on that one shape. Merging its
    // methods into `Arc` would put them on every `Arc` in the crate.
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Inner<T> { pub v: T }\n\
         pub struct Other;\n\
         pub trait Signal { fn notify(&self) -> u8; }\n\
         impl<T> Signal for Arc<Inner<T>> { fn notify(&self) -> u8 { 0 } }",
    )]);
    let probe = c.probe("lib.rs");
    let inner = c.named("lib.rs", "Inner", vec![Ty::Prim(Prim::U32)]);
    let wrapped = c.system(ARC, vec![inner]);
    assert!(probe.resolve_method(&wrapped, "notify").is_ok());

    let other = c.system(ARC, vec![c.named("lib.rs", "Other", vec![])]);
    assert!(
        probe.resolve_method(&other, "notify").is_err(),
        "an Arc of something else has no such method"
    );
}

#[test]
fn a_dyn_receiver_dispatches_to_the_traits_own_declaration() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait TContext { fn node_id(&self) -> u32; }\n\
         pub trait Extended: TContext { fn extra(&self) -> u8; }",
    )]);
    let probe = c.probe("lib.rs");
    let trait_id = c
        .reg
        .module_type(c.module("lib.rs"), "TContext")
        .expect("declared");
    let extended = c
        .reg
        .module_type(c.module("lib.rs"), "Extended")
        .expect("declared");
    let object = Ty::Dyn {
        traits: vec![crate::ty::TraitRef {
            id: extended,
            args: Vec::new(),
            bindings: Vec::new(),
        }],
    };
    // A supertrait's method is reached through the object too.
    let found = probe.resolve_method(&object, "node_id").expect("resolves");
    assert_eq!(
        found.callee,
        Callee::TraitObject(trait_id, "node_id".to_string())
    );
    assert_eq!(found.ret, Ty::Prim(Prim::U32));

    assert!(
        probe.resolve_method(&object, "missing").is_err(),
        "a name the trait does not declare is still not found"
    );
}

#[test]
fn a_bounded_parameter_dispatches_through_its_bound() {
    // This is `Self` inside a trait's own default body: a parameter that is
    // known to implement exactly one trait.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Signal { fn id(&self) -> u32; fn twice(&self) -> u32 { 0 } }",
    )]);
    let trait_id = c
        .reg
        .module_type(c.module("lib.rs"), "Signal")
        .expect("declared");
    let bounds = vec![(
        "Self".to_string(),
        crate::ty::TraitRef {
            id: trait_id,
            args: Vec::new(),
            bindings: Vec::new(),
        },
    )];
    let probe = c.probe("lib.rs").with_bounds(&bounds);
    let found = probe
        .resolve_method(&Ty::Param("Self".into()), "id")
        .expect("the bound answers");
    assert_eq!(found.ret, Ty::Prim(Prim::U32));
    assert!(
        probe
            .resolve_method(&Ty::Param("Other".into()), "id")
            .is_err(),
        "a parameter with no bound has no methods"
    );
}

#[test]
fn an_impl_inherits_the_body_the_trait_wrote() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         pub trait Signal { fn id(&self) -> u32; fn twice(&self) -> u32 { 0 } }\n\
         impl Signal for S { fn id(&self) -> u32 { 0 } }",
    )]);
    let s = c.named("lib.rs", "S", vec![]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&s, "twice")
        .expect("the default body's signature answers");
    assert_eq!(found.ret, Ty::Prim(Prim::U32));
}

// ── Associated types ──────────────────────────────────────────────────

#[test]
fn a_projection_is_read_off_the_impl_that_supplies_it() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Holder;\n\
         pub trait Carrier { type Item; fn take(&self) -> Self::Item; }\n\
         impl Carrier for Holder { type Item = u16; fn take(&self) -> Self::Item { 0 } }",
    )]);
    let holder = c.named("lib.rs", "Holder", vec![]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&holder, "take")
        .expect("resolves");
    assert_eq!(
        found.ret,
        Ty::Prim(Prim::U16),
        "`Self::Item` is normalised through the impl"
    );
}

#[test]
fn a_projection_no_impl_supplies_is_left_standing() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Carrier { type Item; }\npub struct Holder;",
    )]);
    let holder = c.named("lib.rs", "Holder", vec![]);
    let projection = Ty::Assoc {
        base: Box::new(holder),
        trait_: None,
        name: "Item".to_string(),
    };
    assert_eq!(
        c.probe("lib.rs").normalize(&projection),
        projection,
        "nothing supplies it, so nothing is substituted for it"
    );
}

// ── Refusals ──────────────────────────────────────────────────────────

#[test]
fn two_answers_at_one_receiver_is_an_ambiguity_naming_both() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         pub trait Left { fn go(&self) -> u8; }\n\
         pub trait Right { fn go(&self) -> u16; }\n\
         impl Left for S { fn go(&self) -> u8 { 0 } }\n\
         impl Right for S { fn go(&self) -> u16 { 0 } }",
    )]);
    // Both traits are declared in this module, so both are in scope and Rust
    // would need a qualified path too.
    let s = c.named("lib.rs", "S", vec![]);
    let err = c
        .probe("lib.rs")
        .resolve_method(&s, "go")
        .expect_err("two impls answer");
    let message = err.describe(&c.reg, "go");
    assert!(message.contains("ambiguous"), "{}", message);
    assert!(message.contains("Left"), "{}", message);
    assert!(message.contains("Right"), "{}", message);
}

#[test]
fn a_method_nothing_declares_names_every_receiver_that_was_tried() {
    let c = Fixture::build(&[("lib.rs", "pub struct Inner;")]);
    let held = c.system(ARC, vec![c.named("lib.rs", "Inner", vec![])]);
    let err = c
        .probe("lib.rs")
        .resolve_method(&held, "frobnicate")
        .expect_err("nothing declares it");
    let message = err.describe(&c.reg, "frobnicate");
    assert!(message.contains("no method `frobnicate`"), "{}", message);
    assert!(
        message.contains("Arc<Inner>") && message.contains("Inner"),
        "the chain it walked is in the message: {}",
        message
    );
}

#[test]
fn a_field_is_reached_through_the_same_chain_as_a_method() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Inner { pub listeners: u8 }\n\
         pub struct Broadcast(Arc<Inner>);",
    )]);
    let broadcast = c.named("lib.rs", "Broadcast", vec![]);
    let probe = c.probe("lib.rs");
    let held = probe.resolve_field(&broadcast, "_0").expect("the tuple field");
    assert!(held.accessors().is_empty());
    let listeners = probe
        .resolve_field(&held.ty, "listeners")
        .expect("through the Arc");
    assert_eq!(listeners.accessors(), vec!["value"]);
    assert_eq!(listeners.ty, Ty::Prim(Prim::U8));
    assert!(probe.resolve_field(&broadcast, "missing").is_none());
}

#[test]
fn a_map_behind_a_guard_answers_with_its_own_method() {
    // The site the whole chain exists for: `self.0.listeners.read().len()`.
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::{Arc, RwLock};\nuse std::collections::HashMap;\n\
         pub struct Inner { pub listeners: RwLock<HashMap<u32, u8>> }\n\
         pub struct Broadcast(Arc<Inner>);",
    )]);
    let broadcast = c.named("lib.rs", "Broadcast", vec![]);
    let probe = c.probe("lib.rs");
    let held = probe.resolve_field(&broadcast, "_0").unwrap();
    let listeners = probe.resolve_field(&held.ty, "listeners").unwrap();
    let read = probe.resolve_method(&listeners.ty, "read").unwrap();
    let guard = probe
        .resolve_method(&read.ret, "unwrap")
        .expect("Result::unwrap on the LockResult `read` hands back");
    let len = probe.resolve_method(&guard.ret, "len").expect("HashMap::len");
    assert_eq!(len.accessors(), vec!["value"]);
    assert_eq!(len.ret, Ty::Prim(Prim::Usize));
    assert_eq!(
        len.receiver_type(),
        &c.system(HASHMAP, vec![Ty::Prim(Prim::U32), Ty::Prim(Prim::U8)]),
        "the translation is chosen by the type the callee is written for"
    );
}

// ── Bounds are proved, not assumed ────────────────────────────────────

#[test]
fn a_bound_is_proved_against_the_traits_arguments_not_only_its_name() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         pub trait Marker<T> {}\n\
         impl Marker<u16> for S {}\n\
         pub trait Narrow { fn go(&self) -> u8; }\n\
         impl<T: Marker<u8>> Narrow for T { fn go(&self) -> u8 { 0 } }\n\
         pub trait Wide { fn wide(&self) -> u8; }\n\
         impl<T: Marker<u16>> Wide for T { fn wide(&self) -> u8 { 0 } }",
    )]);
    let s = c.named("lib.rs", "S", vec![]);
    let probe = c.probe("lib.rs");
    assert!(
        probe.resolve_method(&s, "go").is_err(),
        "`impl Marker<u16> for S` does not prove `S: Marker<u8>`"
    );
    assert!(
        probe.resolve_method(&s, "wide").is_ok(),
        "the arguments agree, so this one does hold"
    );
}

#[test]
fn a_conditional_deref_does_not_apply_where_its_bound_fails() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Yes;\npub struct No;\npub struct Inner;\n\
         pub trait Bound {}\n\
         impl Bound for Yes {}\n\
         pub struct Wrapper<T> { pub v: T }\n\
         impl Inner { pub fn ping(&self) -> u8 { 0 } }\n\
         impl<T: Bound> std::ops::Deref for Wrapper<T> {\n\
           type Target = Inner;\n\
           fn deref(&self) -> &Inner { todo!() }\n\
         }",
    )]);
    let probe = c.probe("lib.rs");
    let held = |name: &str| c.named("lib.rs", "Wrapper", vec![c.named("lib.rs", name, vec![])]);
    assert!(
        probe.resolve_method(&held("Yes"), "ping").is_ok(),
        "the bound holds, so the wrapper dereferences"
    );
    assert!(
        probe.resolve_method(&held("No"), "ping").is_err(),
        "the bound fails, so there is nothing behind the wrapper"
    );
}

#[test]
fn a_bound_written_on_a_parameter_in_scope_is_its_own_proof() {
    // `impl<SE: StorageEngine> Deref for Node<SE>` applies inside a function
    // that declares the same bound; there is no impl to go looking for.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Engine {}\npub struct Inner;\n\
         impl Inner { pub fn ping(&self) -> u8 { 0 } }\n\
         pub struct Node<SE> { pub v: SE }\n\
         impl<SE: Engine> std::ops::Deref for Node<SE> {\n\
           type Target = Inner;\n\
           fn deref(&self) -> &Inner { todo!() }\n\
         }",
    )]);
    let trait_id = c.reg.module_type(c.module("lib.rs"), "Engine").unwrap();
    let bounds = vec![(
        "SE".to_string(),
        crate::ty::TraitRef { id: trait_id, args: Vec::new(), bindings: Vec::new() },
    )];
    let node = c.named("lib.rs", "Node", vec![Ty::Param("SE".into())]);
    assert!(
        c.probe("lib.rs").with_bounds(&bounds).resolve_method(&node, "ping").is_ok(),
        "the bound in scope proves it"
    );
    assert!(
        c.probe("lib.rs").resolve_method(&node, "ping").is_err(),
        "without the bound in scope there is nothing to prove it with"
    );
}

#[test]
fn clone_is_answered_by_a_registered_impl_and_refused_otherwise() {
    let c = Fixture::build(&[(
        "lib.rs",
        "#[derive(Clone)]\npub struct Copied;\npub struct Plain;",
    )]);
    let probe = c.probe("lib.rs");
    let copied = c.named("lib.rs", "Copied", vec![]);
    let found = probe.resolve_method(&copied, "clone").expect("the derive registers it");
    assert_eq!(found.ret, copied);
    assert!(
        probe
            .resolve_method(&c.named("lib.rs", "Plain", vec![]), "clone")
            .is_err(),
        "nothing says this type is cloneable, and saying so anyway bypassed the sink"
    );
}

#[test]
fn a_sole_candidate_from_a_trait_out_of_scope_is_taken_and_reported() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod hidden;\npub mod here;\npub struct Conc;"),
        (
            "hidden.rs",
            "use crate::Conc;\n\
             pub trait Only { fn tag(&self) -> u8; }\n\
             impl Only for Conc { fn tag(&self) -> u8 { 0 } }",
        ),
        ("here.rs", "use crate::Conc;"),
    ]);
    let conc = c.named("lib.rs", "Conc", vec![]);
    let found = c
        .probe("here.rs")
        .resolve_method(&conc, "tag")
        .expect("the method is taken rather than deleted");
    assert!(
        found.out_of_scope.is_some(),
        "and the fact that Rust would not admit it is recorded"
    );
    assert!(
        c.probe("hidden.rs").resolve_method(&conc, "tag").unwrap().out_of_scope.is_none(),
        "where the trait is nameable there is nothing to report"
    );
}

#[test]
fn an_undecidable_bound_travels_with_the_answer_rather_than_being_assumed() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Show { fn show(&self) -> u8; }\n\
         impl<T: nowhere::Shown> Show for T { fn show(&self) -> u8 { 0 } }",
    )]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&Ty::Prim(Prim::I16), "show")
        .expect("the impl is still a candidate");
    assert_eq!(found.obligations.len(), 1);
    assert_eq!(found.obligations[0].reason, Undecided::NoDeclaration);
}

#[test]
fn a_nested_undecidable_bound_is_not_swallowed() {
    // `T: Outer` needs `T: Inner`, and `Inner` has no declaration. Reporting the
    // outer bound as proven would rest it on a question nobody answered.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         pub trait Outer {}\n\
         impl<T: nowhere::Shown> Outer for T {}\n\
         pub trait Show { fn show(&self) -> u8; }\n\
         impl<T: Outer> Show for T { fn show(&self) -> u8 { 0 } }",
    )]);
    let s = c.named("lib.rs", "S", vec![]);
    let found = c.probe("lib.rs").resolve_method(&s, "show").expect("still a candidate");
    assert_eq!(
        found.obligations.len(),
        1,
        "the inner question surfaces as the outer one being undecided"
    );
    assert_eq!(found.obligations[0].reason, Undecided::NoDeclaration);
}
