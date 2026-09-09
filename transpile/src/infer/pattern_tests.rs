//! What a PATTERN says about the value it matches, and what it binds.

use crate::testing::Fixture;
use crate::ty::{Prim, Ty};

const OPTION: &str = "std::option::Option";

fn expr(src: &str) -> syn::Expr {
    syn::parse_str(src).expect("parses as an expression")
}

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
fn a_pattern_says_what_the_value_it_matches_is() {
    // `match found { Some(_) => .. }` says `found` is an `Option`, which is
    // what settles a scrutinee nothing else has typed.
    let c = Fixture::build(&[("lib.rs", "pub struct S { pub slot: Option<u32> }")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);

    let pat: syn::Pat = syn::parse::Parser::parse_str(syn::Pat::parse_single, "Some(inner)")
        .expect("parses as a pattern");
    let shape = cx.pattern_shape(&pat).expect("a variant names its enum");
    let Ty::Named { id, args } = &shape else { panic!("a variant names a type") };
    assert_eq!(*id, c.system_id(OPTION));
    assert_eq!(args.len(), 1, "the argument the pattern does not say is an unknown");
    assert!(matches!(args[0], Ty::Var(_)));

    // A binding names no type, so it says nothing about what it matches.
    let bare: syn::Pat = syn::parse::Parser::parse_str(syn::Pat::parse_single, "other")
        .expect("parses as a pattern");
    assert_eq!(cx.pattern_shape(&bare), None);
}

#[test]
fn a_match_takes_its_type_from_the_arms_own_binding() {
    // `Some(buffer)` shadows the outer `buffer`. Reading the outer one made the
    // arm's call take an `Option<&Vec<u8>>` where it declares `&Vec<u8>`, and
    // the engine reported valid Rust as contradicting itself.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u32 }\n\
         impl Tag {\n    \
             pub fn new() -> Tag { Tag { n: 0 } }\n    \
             pub fn from_buffer(buffer: &Vec<u8>) -> Tag { Tag { n: buffer.len() as u32 } }\n\
         }\n\
         pub fn build(buffer: Option<&Vec<u8>>) -> u32 {\n    \
             let made = match buffer {\n        \
                 Some(buffer) => Tag::from_buffer(buffer),\n        \
                 None => Tag::new(),\n    \
             };\n    \
             made.n\n\
         }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(
        !c.messages()
            .iter()
            .any(|m| m.contains("constrained to be the same type")),
        "{:?}",
        c.messages()
    );
}

#[test]
fn a_ranges_end_settles_the_literal_its_start_wrote() {
    // The endpoints are one type, read by the ordinary rules. Reading the start
    // as `usize` because it is an unsuffixed literal made `for i in 0..4u8` an
    // iteration over `usize`, and the `u8` the body then met a contradiction.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub fn widths() -> Vec<u8> {\n    \
             let mut xs = Vec::new();\n    \
             for i in 0..4u8 { xs.push(i); }\n    \
             xs\n\
         }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(
        !c.messages()
            .iter()
            .any(|m| m.contains("constrained to be the same type")),
        "{:?}",
        c.messages()
    );
}

#[test]
fn a_range_of_two_literals_takes_the_width_the_body_asks_for() {
    // Neither endpoint says a width, so the body does: `add` declares `u32` and
    // the loop hands it `u32`, rather than the `usize` an index position would
    // give or the `i32` a literal nothing decided falls back to.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Store { pub ids: Vec<u32> }\n\
         impl Store {\n    \
             pub fn new() -> Store { Store { ids: Vec::new() } }\n    \
             pub fn add(&mut self, id: u32) { self.ids.push(id); }\n\
         }\n\
         pub fn roots() -> usize {\n    \
             let mut store = Store::new();\n    \
             for id in 1..=6 { store.add(id); }\n    \
             store.ids.len()\n\
         }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(
        !c.messages()
            .iter()
            .any(|m| m.contains("constrained to be the same type")),
        "{:?}",
        c.messages()
    );
}
