//! What the solve does with a constraint that has no solution, and with a
//! literal nothing decided.

use syn::visit::Visit;

use crate::infer::TypeContext;
use crate::testing::Fixture;
use crate::ty::{InferTable, Prim, Ty};

fn contradictions(c: &Fixture) -> Vec<String> {
    c.messages()
        .into_iter()
        .filter(|m| m.contains("constrained to be the same type here and cannot be"))
        .collect()
}

/// Every unsuffixed integer literal in a block, in source order.
struct Literals(Vec<proc_macro2::Span>);

impl<'ast> Visit<'ast> for Literals {
    fn visit_lit_int(&mut self, lit: &'ast syn::LitInt) {
        if lit.suffix().is_empty() {
            self.0.push(syn::spanned::Spanned::span(lit));
        }
    }
}

/// The unknown standing at one written site, as the table left it.
fn at_site(table: &InferTable, span: proc_macro2::Span) -> Ty {
    let at = span.start();
    Ty::Var(
        table
            .site(at.line, at.column, 0)
            .expect("a variable stands at this site"),
    )
}

fn solve(cx: &mut TypeContext<'_>, source: &str) -> syn::Block {
    let block: syn::Block = syn::parse_str(source).unwrap();
    cx.collect_constraints(&block, None);
    block
}

#[test]
fn a_constraint_that_contradicts_an_earlier_binding_is_said_once() {
    // `xs.push(1u32)` decides the element and `xs.push("x")` disagrees. The
    // walk that collects constraints is quiet, so the report survives it only
    // by being re-filed from the table.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub fn count() -> usize {\n    \
             let mut xs = Vec::new();\n    \
             xs.push(1u32);\n    \
             xs.push(\"x\");\n    \
             xs.len()\n\
         }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

#[test]
fn what_a_contradiction_touched_stands_for_nothing() {
    // The first binding does not win: the element the two pushes disagree
    // about is one the engine cannot defend, so the collection is untyped and
    // releases nothing rather than releasing what came first.
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let block = solve(
        &mut cx,
        "{ let mut xs = Vec::new(); xs.push(1u32); xs.push(\"x\"); }",
    );
    let element = match &block.stmts[0] {
        syn::Stmt::Local(local) => {
            at_site(&cx.vars.borrow(), syn::spanned::Spanned::span(&local.init.as_ref().unwrap().expr))
        }
        other => panic!("{:?}", other),
    };
    assert_eq!(cx.solved(&element), element, "the element still answers");
}

#[test]
fn a_constraint_a_later_round_meets_is_not_a_contradiction() {
    // The call is read before the push that decides the element, so the first
    // round refuses what the second round meets; only what still has no
    // solution at the fixed point is a fact about the body.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Thing;\n\
         fn take(values: &Vec<Thing>) -> usize { values.len() }\n\
         pub fn count() -> usize {\n    \
             let mut xs = Vec::new();\n    \
             let n = take(&xs);\n    \
             xs.push(Thing);\n    \
             n\n\
         }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}

#[test]
fn a_value_and_what_it_derefs_to_do_not_disagree() {
    // `&String` stands where `&str` is declared: Rust derefs one into the
    // other and the port writes both as one value.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "fn take(text: &str) -> usize { text.len() }\n\
         pub fn count(owned: String) -> usize { take(&owned) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}

#[test]
fn an_unsuffixed_literal_takes_the_width_a_later_statement_gives_it() {
    // `let mut n = 0` is Rust's `{integer}`: `n = xs.len()` decides it, so the
    // arithmetic below is `usize` and not the `i32` a bare literal defaults to.
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![(
        "xs".into(),
        c.system("std::vec::Vec", vec![Ty::Prim(Prim::U8)]),
    )]);
    let block = solve(&mut cx, "{ let mut n = 0; n = xs.len(); n + 1 }");
    let mut found = Literals(Vec::new());
    found.visit_block(&block);
    assert_eq!(
        cx.solved(&at_site(&cx.vars.borrow(), found.0[0])),
        Ty::Prim(Prim::Usize)
    );
}

#[test]
fn an_unsuffixed_literal_nothing_decides_is_the_width_rust_defaults_to() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let block = solve(&mut cx, "{ let n = 0; n }");
    let mut found = Literals(Vec::new());
    found.visit_block(&block);
    assert_eq!(
        cx.solved(&at_site(&cx.vars.borrow(), found.0[0])),
        Ty::Prim(Prim::I32)
    );
}

#[test]
fn a_literal_may_not_stand_for_a_type_that_is_not_a_number() {
    // Rust's `{integer}` binds only an integer, which is what keeps an
    // unrelated constraint from making a literal the unit type.
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let block = solve(&mut cx, "{ let n = 0; let m: () = n; m }");
    let mut found = Literals(Vec::new());
    found.visit_block(&block);
    assert_eq!(
        cx.solved(&at_site(&cx.vars.borrow(), found.0[0])),
        Ty::Prim(Prim::I32)
    );
}

#[test]
fn a_const_written_inside_a_body_types_the_names_below_it() {
    // The walk that collects constraints has to bind it too: left to the walk
    // that writes the body, the two disagree about what `n` holds.
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let block = solve(&mut cx, "{ const MAX: usize = 3; let mut n = 0; n = MAX; n }");
    let mut found = Literals(Vec::new());
    found.visit_block(&block);
    assert_eq!(
        cx.solved(&at_site(&cx.vars.borrow(), found.0[1])),
        Ty::Prim(Prim::Usize)
    );
}

#[test]
fn two_declared_reference_layers_over_a_value_carrying_none_disagree() {
    // One declared `&` over a value carrying none is the borrow the engine
    // reads that value through. Two is a type the caller never wrote, and Rust
    // rejects the call.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Sink;\n\
         impl Sink {\n    pub fn take(&self, value: &&u8) -> usize { 0 }\n}\n\
         pub fn go(sink: &Sink, n: u8) -> usize { sink.take(n) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

#[test]
fn one_declared_reference_layer_is_the_borrow_the_engine_reads_through() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Sink;\n\
         impl Sink {\n    pub fn take(&self, value: &u8) -> usize { 0 }\n}\n\
         pub fn go(sink: &Sink, n: u8) -> usize { sink.take(n) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}

#[test]
fn a_deref_inside_a_type_argument_is_not_a_coercion() {
    // Rust coerces where a value meets a declared type and nowhere inside one:
    // a `Box<Expr>` stands where an `Expr` is declared, and a `Vec<Box<Expr>>`
    // does not stand where a `Vec<Expr>` is.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Expr;\n\
         pub fn take(values: Vec<Expr>) -> usize { values.len() }\n\
         pub fn go(boxed: Vec<Box<Expr>>) -> usize { take(boxed) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

#[test]
fn a_deref_where_a_value_meets_a_declared_type_is_one() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Expr;\n\
         pub fn take(value: Expr) -> usize { 0 }\n\
         pub fn go(boxed: Box<Expr>) -> usize { take(boxed) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}

#[test]
fn a_sequence_whose_elements_differ_is_a_contradiction() {
    // A `Vec<T>` stands where a slice of `T` is declared, so what the two say
    // about each other is their ELEMENT — and an element that cannot be the
    // one declared is as much a contradiction as the whole would be.
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag;\n\
         pub fn take(values: &[u32]) -> usize { values.len() }\n\
         pub fn go(tags: Vec<Tag>) -> usize { take(&tags) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

#[test]
fn a_box_is_a_sequence_only_of_what_it_boxes_a_slice_of() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub fn take(values: &[u8]) -> usize { values.len() }\n\
         pub fn boxed(one: Box<u8>) -> usize { take(&one) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert_eq!(contradictions(&c).len(), 1, "{:?}", c.messages());
}

#[test]
fn a_boxed_slice_stands_where_a_slice_is_declared() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub fn take(values: &[u8]) -> usize { values.len() }\n\
         pub fn boxed(many: Box<[u8]>) -> usize { take(&many) }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(contradictions(&c).is_empty(), "{:?}", c.messages());
}
