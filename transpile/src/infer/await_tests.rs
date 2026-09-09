//! What `.await` answers with, and when it answers nothing.

use crate::testing::Fixture;
use crate::ty::{Prim, Ty};

fn expr(src: &str) -> syn::Expr {
    syn::parse_str(src).expect("parses as an expression")
}

#[test]
fn awaiting_a_call_to_an_async_fn_is_the_identity() {
    // This port models a call to an `async fn` as returning what it writes
    // rather than a future (spec 4.10), so the `await` adds nothing.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub async fn width() -> u32 { 3 }\npub struct S;",
    )]);
    let cx = c.context("lib.rs", None);
    assert_eq!(
        cx.resolve_expr(&expr("width().await")).unwrap(),
        Ty::Prim(Prim::U32)
    );
}

#[test]
fn awaiting_a_value_with_no_readable_future_is_refused() {
    // Taking the base as the answer typed the site as the thing being awaited,
    // which is the value the position wanted compared against a wrapper.
    let c = Fixture::build(&[("lib.rs", "pub struct Held { pub n: u32 }")]);
    let cx = c.context("lib.rs", None);
    let mut cx = cx;
    cx.push_fn(vec![("held".into(), c.named("lib.rs", "Held", vec![]))]);
    let err = cx.resolve_expr(&expr("held.await")).unwrap_err();
    assert_eq!(
        err.message,
        "`.await` on `Held`, whose `Future` the engine cannot read"
    );
}
