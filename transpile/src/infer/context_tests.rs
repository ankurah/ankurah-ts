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
fn read_unwrap_types_to_the_guard_through_lock_result() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::RwLock;\npub struct S { pub cell: RwLock<Option<u32>> }",
    )]);
    let self_ty = c.named("lib.rs", "S", vec![]);
    let mut cx = c.context("lib.rs", Some(self_ty));
    cx.push_fn(vec![]);

    // `RwLock::read` yields a `LockResult<RwLockReadGuard<'_, T>>`, which is
    // what the std surface declares, so `read().unwrap()` is `Result::unwrap`
    // and hands back the guard. There is no shim: the engine walks the same
    // impls Rust does. (The port's `RwLock.read()` yields the guard directly;
    // that is an emission fact, and the `.unwrap()` written on a `LockResult`
    // receiver emits nothing.)
    let guard = c.system(
        "std::sync::RwLockReadGuard",
        vec![c.system(OPTION, vec![Ty::Prim(Prim::U32)])],
    );
    let lock_result = c.system(
        "std::result::Result",
        vec![
            guard.clone(),
            c.system("std::sync::PoisonError", vec![guard.clone()]),
        ],
    );
    assert_eq!(
        cx.resolve_expr(&expr("self.cell.read()")).unwrap(),
        lock_result,
        "the alias `LockResult<Guard>` expands to the Result it is"
    );
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
    assert_eq!(err.message, "no method `frobnicate` on `Option<u32>`; tried `Option<u32>`");
    assert_eq!(err.file, "lib.rs");

    let err = cx.resolve_expr(&expr("self.missing")).unwrap_err();
    assert_eq!(err.message, "no field `missing` on `S`");
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
    assert!(
        c.probe("stack.rs")
            .deref_once(&c.system(REFCELL, vec![Ty::Prim(Prim::U32)]))
            .is_none(),
        "a RefCell is not reached through, its guard is"
    );
}

// ── Pattern binding ───────────────────────────────────────────────────
//
// A `match` arm, an `if let`, a `for` loop and a destructuring `let` all
// introduce names, and every one of them has a type the value being taken apart
// already knows. Before these bound anything, each use of such a name was a
// name the engine could not find at all.

fn pat(src: &str) -> syn::Pat {
    // A match arm takes every pattern form, including the `|` a `let` refuses.
    let matched: syn::Expr =
        syn::parse_str(&format!("match () {{ {} => () }}", src)).expect("parses as a pattern");
    match matched {
        syn::Expr::Match(m) => m.arms.into_iter().next().expect("one arm").pat,
        _ => unreachable!(),
    }
}

/// Bind a pattern against a type and read one of the names back.
fn bound(c: &Fixture, cx: &mut crate::infer::TypeContext<'_>, src: &str, ty: &Ty, name: &str) -> Option<Ty> {
    let _ = c;
    cx.bind_pattern(&pat(src), Some(ty));
    cx.resolve_expr(&expr(name)).ok()
}

#[test]
fn a_tuple_pattern_types_each_position() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let pair = Ty::Tuple(vec![Ty::Prim(Prim::U8), Ty::Prim(Prim::Bool)]);
    assert_eq!(bound(&c, &mut cx, "(a, b)", &pair, "a"), Some(Ty::Prim(Prim::U8)));
    assert_eq!(cx.resolve_expr(&expr("b")).unwrap(), Ty::Prim(Prim::Bool));

    // A tuple of the wrong width says nothing about either name rather than
    // pairing them up wrongly.
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let single = Ty::Tuple(vec![Ty::Prim(Prim::U8)]);
    assert_eq!(bound(&c, &mut cx, "(a, b)", &single, "a"), None);
}

#[test]
fn a_struct_pattern_types_each_field_and_a_variant_its_payload() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Point { pub x: u8, pub y: bool }\n\
         pub enum Shape { Dot(u16), Named { label: u32 } }",
    )]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);

    let point = c.named("lib.rs", "Point", vec![]);
    assert_eq!(
        bound(&c, &mut cx, "Point { x, y }", &point, "x"),
        Some(Ty::Prim(Prim::U8))
    );
    assert_eq!(cx.resolve_expr(&expr("y")).unwrap(), Ty::Prim(Prim::Bool));

    let shape = c.named("lib.rs", "Shape", vec![]);
    assert_eq!(
        bound(&c, &mut cx, "Shape::Dot(n)", &shape, "n"),
        Some(Ty::Prim(Prim::U16))
    );
    assert_eq!(
        bound(&c, &mut cx, "Shape::Named { label }", &shape, "label"),
        Some(Ty::Prim(Prim::U32))
    );
}

#[test]
fn a_generic_enums_payload_is_substituted_through_the_scrutinee() {
    let c = Fixture::build(&[("lib.rs", "pub enum Slot<T> { Full(T), Empty }")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let slot = c.named("lib.rs", "Slot", vec![Ty::Prim(Prim::U64)]);
    assert_eq!(
        bound(&c, &mut cx, "Slot::Full(v)", &slot, "v"),
        Some(Ty::Prim(Prim::U64))
    );
    // A unit variant written as a bare name is the variant, not a new binding.
    cx.bind_pattern(&pat("Empty"), Some(&slot));
    assert!(cx.resolve_expr(&expr("Empty")).is_err());
}

#[test]
fn some_ok_and_err_take_apart_the_system_types() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);

    let option = c.system(OPTION, vec![Ty::Prim(Prim::U8)]);
    assert_eq!(
        bound(&c, &mut cx, "Some(v)", &option, "v"),
        Some(Ty::Prim(Prim::U8))
    );

    let result = c.system(
        "std::result::Result",
        vec![Ty::Prim(Prim::U16), Ty::Prim(Prim::Bool)],
    );
    assert_eq!(
        bound(&c, &mut cx, "Ok(v)", &result, "v"),
        Some(Ty::Prim(Prim::U16))
    );
    assert_eq!(
        bound(&c, &mut cx, "Err(e)", &result, "e"),
        Some(Ty::Prim(Prim::Bool))
    );
}

#[test]
fn a_reference_pattern_and_an_or_pattern_bind_the_same_names() {
    let c = Fixture::build(&[("lib.rs", "pub enum Side { Left(u8), Right(u8) }")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let side = Ty::Ref {
        mutable: false,
        inner: Box::new(c.named("lib.rs", "Side", vec![])),
    };
    // Matching a non-reference pattern against a `&Side` peels one layer and
    // binds everything under it by reference, which is what Rust does: `n` is a
    // `&u8`, not a `u8`.
    assert_eq!(
        bound(&c, &mut cx, "Side::Left(n) | Side::Right(n)", &side, "n"),
        Some(Ty::Ref {
            mutable: false,
            inner: Box::new(Ty::Prim(Prim::U8))
        }),
        "each alternative binds the same name against the same value"
    );
}

#[test]
fn a_name_a_pattern_could_not_type_is_bound_and_says_so() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let untyped = cx.bind_pattern(&pat("(a, b)"), None);
    assert_eq!(untyped, vec!["a".to_string(), "b".to_string()]);
    let err = cx.resolve_expr(&expr("a")).unwrap_err();
    assert_eq!(
        err.message, "`a` is bound here but the engine could not type it",
        "a bound name with no type is a different gap from a name nothing binds"
    );
}

#[test]
fn a_for_loop_binds_the_element_of_what_it_iterates() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let vec = c.system("std::vec::Vec", vec![Ty::Prim(Prim::U8)]);
    assert_eq!(cx.iteration_item(&vec), Some(Ty::Prim(Prim::U8)));
    let map = c.system(
        "std::collections::HashMap",
        vec![Ty::Prim(Prim::U32), Ty::Prim(Prim::Bool)],
    );
    assert_eq!(
        cx.iteration_item(&map),
        Some(Ty::Tuple(vec![Ty::Prim(Prim::U32), Ty::Prim(Prim::Bool)]))
    );
    // Anything the declared surface does not cover is refused rather than
    // guessed at; the loop variable is then bound without a type.
    assert_eq!(cx.iteration_item(&c.named("lib.rs", "S", vec![])), None);
}

#[test]
fn a_match_arm_binds_its_payload_for_the_body_it_guards() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Inner { pub count: u8 }\n\
         pub enum Held { One(Arc<Inner>), Nothing }\n\
         pub struct S { pub held: Held }\n\
         impl S {\n\
           pub fn read(&self) -> u8 {\n\
             match &self.held { Held::One(inner) => inner.count, Held::Nothing => 0 }\n\
           }\n\
         }",
    )]);
    let body = c.translated_method("lib.rs", "read");
    // The arm's payload is an `Arc`, so reaching its field writes the accessor.
    // The name itself is rewritten to the payload slot by the match translation,
    // which is why the assertion is on the accessor and not on `inner`.
    assert!(
        body.contains(".value.count"),
        "the arm's binding was not typed: {}",
        body
    );
}

#[test]
fn an_if_let_binds_its_payload_for_the_branch_it_guards() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Inner { pub count: u8 }\n\
         pub struct S { pub held: Option<Arc<Inner>> }\n\
         impl S {\n\
           pub fn read(&self) -> u8 {\n\
             if let Some(inner) = &self.held { inner.count } else { 0 }\n\
           }\n\
         }",
    )]);
    let body = c.translated_method("lib.rs", "read");
    assert!(
        body.contains("inner.value.count"),
        "the branch's binding is an Arc: {}",
        body
    );
}

#[test]
fn self_in_a_trait_default_body_is_the_implementor() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Signal { fn id(&self) -> u32; fn twice(&self) -> u32 { self.id() } }",
    )]);
    assert!(
        !c.messages().iter().any(|m| m.contains("outside an impl")),
        "{:?}",
        c.messages()
    );
}

#[test]
fn a_shadowing_let_reads_its_initialiser_in_the_scope_it_shadows() {
    // `let stack = stack.borrow_mut()` borrows the outer `stack`, which is the
    // `RefCell`. Binding the name before translating the initialiser made the
    // receiver the guard the line is about to introduce, and the call reached
    // through it: `stack.value.borrowMut()`.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "use std::cell::RefCell;\n\
         pub struct S { pub cell: RefCell<u32> }\n\
         impl S {\n\
           pub fn go(&self) -> u32 {\n\
             let cell = &self.cell;\n\
             let cell = cell.borrow();\n\
             0\n\
           }\n\
         }",
    )]);
    let body = c.translated_method("lib.rs", "go");
    assert!(
        !body.contains("cell.value.borrow()"),
        "the initialiser is read before the name is rebound: {}",
        body
    );
    // And the shadow is a new variable, so it gets a new identifier rather than
    // overwriting the one it shadows.
    assert!(
        body.contains("const cell_1 = cell.borrow();"),
        "the shadow is declared, not assigned: {}",
        body
    );
}

#[test]
fn a_pattern_matched_against_a_reference_binds_through_it() {
    let c = Fixture::build(&[("lib.rs", "pub struct Point { pub x: u8, pub y: bool }")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let point = c.named("lib.rs", "Point", vec![]);
    let by_reference = Ty::Ref {
        mutable: false,
        inner: Box::new(point.clone()),
    };
    let shared = |ty: Ty| Ty::Ref {
        mutable: false,
        inner: Box::new(ty),
    };

    // Default binding mode: one layer peeled, everything under it by reference.
    assert_eq!(
        bound(&c, &mut cx, "Point { x, y }", &by_reference, "x"),
        Some(shared(Ty::Prim(Prim::U8)))
    );

    // `&mut` gives `&mut`, and a `&` outside a `&mut` still only lends.
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let by_unique = Ty::Ref {
        mutable: true,
        inner: Box::new(point.clone()),
    };
    assert_eq!(
        bound(&c, &mut cx, "Point { x, y }", &by_unique, "x"),
        Some(Ty::Ref {
            mutable: true,
            inner: Box::new(Ty::Prim(Prim::U8))
        })
    );

    // An explicit `&pat` consumes the layer itself, so the mode starts again.
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    assert_eq!(
        bound(&c, &mut cx, "&Point { x, y }", &by_reference, "x"),
        Some(Ty::Prim(Prim::U8))
    );

    // And `ref x` says the borrow outright, whatever the mode was.
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    assert_eq!(
        bound(&c, &mut cx, "Point { x: ref x, y }", &point, "x"),
        Some(shared(Ty::Prim(Prim::U8)))
    );
}

#[test]
fn iterating_a_borrowed_collection_hands_out_borrowed_items() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let vec = c.system("std::vec::Vec", vec![Ty::Prim(Prim::U8)]);
    assert_eq!(cx.iteration_item(&vec), Some(Ty::Prim(Prim::U8)));
    assert_eq!(
        cx.iteration_item(&Ty::Ref {
            mutable: false,
            inner: Box::new(vec)
        }),
        Some(Ty::Ref {
            mutable: false,
            inner: Box::new(Ty::Prim(Prim::U8))
        }),
        "`for x in &v` binds a reference to the element"
    );
}

#[test]
fn a_field_missing_from_a_known_type_is_reported_with_both_names() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct S { pub here: u8 }\n\
         impl S { pub fn go(&self) -> u8 { self.missing } }",
    )]);
    let _ = c.translated_method("lib.rs", "go");
    assert!(
        c.messages()
            .iter()
            .any(|m| m.contains("no field `missing` on `S`")),
        "a known base with no such field says which type and which field: {:?}",
        c.messages()
    );
}

#[test]
fn a_variant_written_as_a_qualified_path_resolves_through_its_enum() {
    // `ast::Literal::I64(v)` and `crate::ast::Literal::I64(v)` name the same
    // variant as `Literal::I64(v)`; resolving only the last segment, or falling
    // through to the capitalisation guess, gets a different answer for each.
    let mut c = Fixture::build_named(
        "testcrate",
        &[
            ("lib.rs", "pub mod ast;\npub mod use_it;"),
            ("ast.rs", "pub enum Literal { I64(i64), Str(String) }"),
            (
                "use_it.rs",
                "use crate::ast;\n\
                 pub fn make(v: i64) -> ast::Literal { ast::Literal::I64(v) }\n\
                 pub fn make_full(v: i64) -> crate::ast::Literal { crate::ast::Literal::I64(v) }",
            ),
        ],
    );
    let cx = c.context("use_it.rs", None);
    let literal = c.named("ast.rs", "Literal", vec![]);
    for written in ["ast::Literal::I64(1)", "crate::ast::Literal::I64(1)"] {
        assert_eq!(
            cx.resolve_expr(&expr(written)).unwrap(),
            literal,
            "{} is a Literal",
            written
        );
    }
    drop(cx);
    let body = c.translated_method("use_it.rs", "make");
    // The variant constructor, not the PascalCase-call guess. How the module
    // qualifier in front of the name is written is the import layer's business.
    assert!(
        body.contains("Literal('I64', { _0: v })"),
        "and it is emitted as the variant constructor: {}",
        body
    );
}
