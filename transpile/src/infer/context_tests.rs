//! What the body translator asks the engine, and what it gets back.

use crate::testing::Fixture;
use crate::ty::{Prim, Ty};

fn expr(src: &str) -> syn::Expr {
    syn::parse_str(src).expect("parses as an expression")
}

const OPTION: &str = "std::option::Option";
const REFCELL: &str = "std::cell::RefCell";
const CELL_REF: &str = "std::cell::Ref";

#[test]
fn expect_on_an_option_yields_the_value_type() {
    let c = Fixture::build(&[("lib.rs", "pub struct S { pub slot: Option<u32> }")]);
    let self_ty = c.named("lib.rs", "S", vec![]);
    let mut cx = c.context("lib.rs", Some(self_ty));
    cx.push_fn(vec![]);

    assert_eq!(
        cx.resolve_expr(&expr("self.slot")).unwrap(),
        c.system(OPTION, vec![Ty::Prim(Prim::U32)])
    );
    // The declared method answers, so the `Option` is gone by the time the
    // value is used — it used to come back as `Option<u32>` again.
    assert_eq!(
        cx.resolve_expr(&expr("self.slot.expect(\"x\")")).unwrap(),
        Ty::Prim(Prim::U32)
    );
    assert_eq!(
        cx.resolve_expr(&expr("self.slot.unwrap()")).unwrap(),
        Ty::Prim(Prim::U32)
    );
    assert_eq!(
        cx.resolve_expr(&expr("self.slot.is_none()")).unwrap(),
        Ty::Prim(Prim::Bool)
    );
}

#[test]
fn unwrap_on_a_lock_guard_keeps_the_guard() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::RwLock;\npub struct S { pub cell: RwLock<Option<u32>> }",
    )]);
    let self_ty = c.named("lib.rs", "S", vec![]);
    let mut cx = c.context("lib.rs", Some(self_ty));
    cx.push_fn(vec![]);

    // `RwLock::read` yields a `LockResult` in Rust and the guard itself in the
    // polyfill, so the `unwrap` the source writes must not reach through the
    // guard and unwrap the `Option` inside it.
    let guard = c.system(
        "std::sync::RwLockReadGuard",
        vec![c.system(OPTION, vec![Ty::Prim(Prim::U32)])],
    );
    assert_eq!(cx.resolve_expr(&expr("self.cell.read()")).unwrap(), guard);
    assert_eq!(
        cx.resolve_expr(&expr("self.cell.read().unwrap()")).unwrap(),
        guard
    );
    // And reading through the guard still finds the Option's own method.
    assert_eq!(
        cx.resolve_expr(&expr("self.cell.read().unwrap().is_none()"))
            .unwrap(),
        Ty::Prim(Prim::Bool)
    );
}

#[test]
fn a_method_the_engine_cannot_find_is_refused_with_its_position() {
    let c = Fixture::build(&[("lib.rs", "pub struct S { pub slot: Option<u32> }")]);
    let self_ty = c.named("lib.rs", "S", vec![]);
    let mut cx = c.context("lib.rs", Some(self_ty));
    cx.push_fn(vec![]);

    let err = cx
        .resolve_expr(&expr("self.slot.frobnicate()"))
        .unwrap_err();
    assert_eq!(err.message, "no method `frobnicate` on this type");
    assert_eq!(err.file, "lib.rs");

    let err = cx.resolve_expr(&expr("self.missing")).unwrap_err();
    assert_eq!(err.message, "no field `missing` on this type");
}

#[test]
fn a_local_takes_its_annotation_and_otherwise_its_initialiser() {
    let c = Fixture::build(&[("lib.rs", "pub struct S { pub slot: Option<u32> }")]);
    let self_ty = c.named("lib.rs", "S", vec![]);
    let mut cx = c.context("lib.rs", Some(self_ty));
    cx.push_fn(vec![]);

    let annotated: syn::Stmt = syn::parse_str("let x: u8 = self.slot.unwrap();").unwrap();
    let syn::Stmt::Local(local) = annotated else {
        panic!("expected a let")
    };
    assert_eq!(cx.resolve_local_type(&local).unwrap(), Ty::Prim(Prim::U8));

    let inferred: syn::Stmt = syn::parse_str("let x = self.slot.unwrap();").unwrap();
    let syn::Stmt::Local(local) = inferred else {
        panic!("expected a let")
    };
    assert_eq!(cx.resolve_local_type(&local).unwrap(), Ty::Prim(Prim::U32));
}

#[test]
fn a_constant_is_reached_through_the_module_that_declares_it() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod policy;"),
        ("policy.rs", "pub const DEFAULT_CONTEXT: u32 = 3;"),
        ("user.rs", "pub struct S;"),
    ]);
    let mut cx = c.context("user.rs", None);
    cx.push_fn(vec![]);
    assert_eq!(
        cx.resolve_expr(&expr("crate::policy::DEFAULT_CONTEXT"))
            .unwrap(),
        Ty::Prim(Prim::U32)
    );
    let err = cx
        .resolve_expr(&expr("crate::policy::MISSING"))
        .unwrap_err();
    assert!(
        err.message.contains("does not name a value"),
        "{}",
        err.message
    );
}

#[test]
fn a_variant_is_recognised_through_its_enum_and_not_by_its_last_segment() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod signal;"),
        ("signal.rs", "pub enum Kind { Constant, Dynamic }"),
        ("user.rs", "use crate::signal::Kind;\npub struct Constant;"),
    ]);
    let cx = c.context("user.rs", None);
    assert!(cx.is_variant("Kind", "Constant"));
    assert!(!cx.is_variant("Kind", "Missing"));
    // `Constant` is a struct here, not an enum, so nothing is a variant of it.
    assert!(!cx.is_variant("Constant", "Constant"));
}

/// The `Ref` fix, checked where it shows: in the emitted TypeScript. A crate
/// type called `Ref` used to delete `std::cell::Ref` from the registry, and
/// with it the `.value` that reaching through a `RefCell` guard needs.
#[test]
fn reaching_through_a_refcell_guard_emits_the_accessor() {
    let mut c = Fixture::build(&[
        ("lib.rs", "pub mod broadcast;\npub mod stack;"),
        // The crate declares its own `Ref`, as signals does.
        ("broadcast.rs", "pub struct Ref<'a, T> { pub inner: &'a T }"),
        (
            "stack.rs",
            "use std::cell::RefCell;\n\
             pub struct Stack { pub entries: RefCell<Vec<u32>> }\n\
             impl Stack {\n\
             pub fn last(&self) -> u32 { *self.entries.borrow().last().unwrap() }\n\
             }",
        ),
    ]);

    let body = c.translated_method("stack.rs", "last");
    assert_eq!(
        body.trim(),
        "return this.entries.borrow().value.at(-1);",
        "the guard's accessor and the Vec translation both fire"
    );

    // Both `Ref`s are still in the registry, each its own type.
    let crate_ref = c.reg.module_type(c.module("broadcast.rs"), "Ref").unwrap();
    assert_ne!(crate_ref, c.system_id(CELL_REF));
    assert_eq!(
        c.reg
            .deref_field(&c.system(REFCELL, vec![Ty::Prim(Prim::U32)])),
        None
    );
}
