//! Where a closure's parameter types come from, one source at a time.

use crate::testing::Fixture;
use crate::ty::{Prim, Ty};

fn closure(src: &str) -> syn::ExprClosure {
    match syn::parse_str::<syn::Expr>(src).expect("parses as an expression") {
        syn::Expr::Closure(c) => c,
        other => panic!("not a closure: {:?}", other),
    }
}

fn expr(src: &str) -> syn::Expr {
    syn::parse_str(src).expect("parses as an expression")
}

/// The types the signature gave each parameter, in order, with a name for the
/// ones it could not type.
fn params(sig: &super::ClosureSig) -> Vec<Option<Ty>> {
    sig.params.iter().map(|(_, ty)| ty.clone()).collect()
}

#[test]
fn an_annotation_on_the_closure_types_its_parameter() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let sig = cx.closure_signature(&closure("|x: u8| x"), None);
    assert_eq!(params(&sig), vec![Some(Ty::Prim(Prim::U8))]);
    assert_eq!(sig.ret, Some(Ty::Prim(Prim::U8)));
}

#[test]
fn a_boxed_fn_at_a_binding_site_types_the_parameter() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let want = c.ty("lib.rs", "Box<dyn Fn(u32) -> bool>");
    let sig = cx.closure_signature(&closure("|x| true"), Some(&want));
    assert_eq!(params(&sig), vec![Some(Ty::Prim(Prim::U32))]);
    assert_eq!(sig.ret, Some(Ty::Prim(Prim::Bool)));
}

#[test]
fn an_impl_fn_parameter_types_the_closure_passed_to_it() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let want = c.ty("lib.rs", "impl FnMut(u16, bool)");
    let sig = cx.closure_signature(&closure("|n, flag| ()"), Some(&want));
    assert_eq!(
        params(&sig),
        vec![Some(Ty::Prim(Prim::U16)), Some(Ty::Prim(Prim::Bool))]
    );
}

#[test]
fn an_arc_of_a_dyn_fn_is_a_callable_the_closure_reads_through() {
    let c = Fixture::build(&[("lib.rs", "use std::sync::Arc;\npub struct S;")]);
    let cx = c.context("lib.rs", None);
    let want = c.ty("lib.rs", "Arc<dyn Fn(u8) + Send + Sync>");
    let sig = cx.closure_signature(&closure("|b| ()"), Some(&want));
    assert_eq!(params(&sig), vec![Some(Ty::Prim(Prim::U8))]);
}

#[test]
fn a_reference_pattern_binds_what_the_reference_points_at() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let want = c.ty("lib.rs", "Box<dyn Fn(&u8)>");
    let sig = cx.closure_signature(&closure("|&x| x"), Some(&want));
    assert_eq!(params(&sig), vec![Some(Ty::Prim(Prim::U8))]);
}

#[test]
fn a_closure_nothing_types_says_which_parameter_it_could_not_read() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let sig = cx.closure_signature(&closure("|x| x"), None);
    assert_eq!(params(&sig), vec![None]);
    assert_eq!(sig.untyped_params(), vec!["x".to_string()]);
}

#[test]
fn a_callees_bound_types_the_closure_it_is_passed() {
    // `Iterator::map` declares `F: FnMut(Self::Item) -> B`, so the element type
    // reaches the closure through the bound and through the projection, neither
    // of which is written at the call.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Id;\npub struct S(pub Vec<Id>);\n\
         impl S { pub fn names(&self) -> Vec<Id> { self.0.iter().map(|id| id).collect() } }",
    )]);
    let self_ty = c.named("lib.rs", "S", vec![]);
    let mut cx = c.context("lib.rs", Some(self_ty));
    cx.push_fn(vec![]);

    let call = expr("self.0.iter().map(|id| id)");
    let syn::Expr::MethodCall(call) = &call else {
        unreachable!()
    };
    let found = cx
        .resolve_method_call_with(&call.receiver, "map", None)
        .expect("map resolves");
    // The bound names `Self::Item`; the probe settles that projection against
    // the receiver, which is what the translator does before it hands the type
    // to the closure.
    let probe = cx.probe();
    let want: Vec<Ty> = c
        .reg
        .method_param_types(&found)
        .iter()
        .map(|ty| probe.normalize(ty))
        .collect();
    let sig = cx.closure_signature(&closure("|id| id"), want.first());
    let id = c.named("lib.rs", "Id", vec![]);
    assert_eq!(
        params(&sig),
        vec![Some(Ty::Ref {
            mutable: false,
            inner: Box::new(id)
        })]
    );
}

#[test]
fn a_closure_the_position_types_has_a_type_of_its_own() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let want = c.ty("lib.rs", "Box<dyn Fn(u32) -> bool>");
    let ty = cx
        .resolve_expr_expecting(&expr("|x| true"), Some(&want))
        .expect("a typed closure has a type");
    let shape = super::fn_shape(&c.reg, &ty, &[]).expect("it is a callable");
    assert_eq!(shape.inputs, vec![Ty::Prim(Prim::U32)]);
    assert_eq!(shape.output, Ty::Prim(Prim::Bool));
}

#[test]
fn a_closure_nothing_types_is_refused_rather_than_guessed() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let refused = cx.resolve_expr(&expr("|x| x")).expect_err("nothing types it");
    assert!(
        refused.message.contains("`x`"),
        "the refusal has to name the parameter: {}",
        refused.message
    );
}

#[test]
fn a_closure_returning_through_its_body_takes_the_tail_type() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let want = c.ty("lib.rs", "Box<dyn Fn(u32) -> bool>");
    // The expected callable says `bool`, and the body agrees; with no expected
    // output at all the body is what answers.
    let open = c.ty("lib.rs", "Box<dyn Fn(u32)>");
    let sig = cx.closure_signature(&closure("|x| x"), Some(&open));
    assert_eq!(sig.ret, Some(Ty::Prim(Prim::U32)));
    let sig = cx.closure_signature(&closure("|x| true"), Some(&want));
    assert_eq!(sig.ret, Some(Ty::Prim(Prim::Bool)));
}

/// A closure parameter written as a TUPLE PATTERN binds one name per element,
/// each with that element's type.
///
/// `|(backend, ops)|` over a map's `iter()` takes one `(&K, &V)`. The signature
/// and the scope were the same list, so the parameter bound one name spelled
/// `[backend, ops]` — which no body ever writes — the closure was reported as
/// "typed by nothing", and `ops.iter()` inside it resolved to nothing. Six
/// sites in proto's `Display` impls, and thirty more across core and
/// storage-common.
#[test]
fn a_tuple_closure_parameter_types_each_name_it_binds() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::collections::BTreeMap;\n\
         pub struct Op { pub diff: Vec<u8> }\n\
         pub fn sizes(m: &BTreeMap<String, Vec<Op>>) -> Vec<String> {\n\
           m.iter().map(|(backend, ops)| format!(\"{} => {}b\", backend, ops.len())).collect()\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "sizes");
    assert!(ts.contains("([backend, ops])"), "{ts}");
    assert!(
        f.messages().iter().all(|m| !m.contains("typed by nothing")),
        "the parameter is typed now: {:?}",
        f.messages()
    );
    assert!(
        f.messages().iter().all(|m| !m.contains("`ops` does not name a value")),
        "and so is every name inside it: {:?}",
        f.messages()
    );
}
