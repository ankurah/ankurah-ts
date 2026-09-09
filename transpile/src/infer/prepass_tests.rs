//! What the walk that collects a body's constraints reads, source by source.

use crate::infer::TypeContext;
use crate::testing::Fixture;
use crate::ty::Ty;

fn expr(src: &str) -> syn::Expr {
    syn::parse_str(src).expect("parses as an expression")
}

/// The type the `let` at the top of this block ends up with.
fn first_local(cx: &mut TypeContext<'_>, source: &str) -> Ty {
    let block: syn::Block = syn::parse_str(source).unwrap();
    cx.collect_constraints(&block, None);
    let syn::Stmt::Local(local) = &block.stmts[0] else { panic!("not a let") };
    let found = cx.resolve_expr(&local.init.as_ref().unwrap().expr).unwrap();
    cx.solved(&found)
}

#[test]
fn an_annotation_settles_what_the_initialiser_left_open() {
    // `Vec::new()` leaves its element to whatever decides it, and the
    // annotation on the `let` is what decides it here.
    let c = Fixture::build(&[("lib.rs", "pub struct Tag;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let found = first_local(&mut cx, "{ let tags: Vec<Tag> = Vec::new(); tags }");
    assert_eq!(
        found,
        c.system("std::vec::Vec", vec![c.named("lib.rs", "Tag", vec![])])
    );
}

#[test]
fn a_macro_written_as_a_statement_constrains_what_is_inside_it() {
    // A macro's arguments are tokens, so no visitor reaches them. The emitter
    // parses them and types what it finds; a walk that does not read the same
    // expressions reads less of the body than the emitter does.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag;\npub fn count(values: &Vec<Tag>) -> usize { values.len() }",
    )]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let found = first_local(
        &mut cx,
        "{ let tags = Vec::new(); assert_eq!(count(&tags), 0); }",
    );
    assert_eq!(
        found,
        c.system("std::vec::Vec", vec![c.named("lib.rs", "Tag", vec![])])
    );
}

#[test]
fn a_borrow_is_a_reference_and_not_what_it_points_at() {
    // What the engine reads as an argument is what the callee is handed. Erased,
    // a `&Literal` met a by-value `From` impl and the wrong conversion was
    // emitted at every such call.
    let c = Fixture::build(&[("lib.rs", "pub struct Tag;")]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    cx.scopes.bind("tag".to_string(), c.named("lib.rs", "Tag", vec![]));
    assert_eq!(
        cx.resolve_expr(&expr("&tag")).unwrap(),
        Ty::Ref {
            mutable: false,
            inner: Box::new(c.named("lib.rs", "Tag", vec![])),
        }
    );
    assert_eq!(
        cx.resolve_expr(&expr("&&tag")).unwrap(),
        Ty::Ref {
            mutable: false,
            inner: Box::new(Ty::Ref {
                mutable: false,
                inner: Box::new(c.named("lib.rs", "Tag", vec![])),
            }),
        }
    );
}

#[test]
fn a_type_parameter_the_callee_owns_never_binds_the_callers_unknown() {
    // `C` names one type per call the callee makes, not one this body can
    // decide; binding it left the caller holding a name nothing declares.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Sink;\n\
         impl Sink {\n    pub fn take<C: Clone>(&self, items: Vec<C>) {}\n}",
    )]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    cx.scopes.bind("sink".to_string(), c.named("lib.rs", "Sink", vec![]));
    let block: syn::Block =
        syn::parse_str("{ let items = Vec::new(); sink.take(items); }").unwrap();
    cx.collect_constraints(&block, None);
    let syn::Stmt::Local(local) = &block.stmts[0] else { panic!("not a let") };
    let found = cx.resolve_expr(&local.init.as_ref().unwrap().expr).unwrap();
    let solved = cx.solved(&found);
    assert!(
        !matches!(&solved, Ty::Named { args, .. } if matches!(args.first(), Some(Ty::Param(_)))),
        "{solved:?}"
    );
}
