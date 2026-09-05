//! Who releases a callable parameter, and who calls it.
//!
//! Rust's `fn f<F: Fn(u32) -> u32>(g: F)` takes `g` BY VALUE: it is dropped at
//! the end of the body, and only the CALL borrows it. The port read those two
//! as one — `invokeRef` for the call, and nothing for the drop — so every
//! capture of every wrapped closure handed to such a parameter leaked. Two live
//! sites: core's `ResultSet::retain_dirty` and signals' `Value::set_with`.

use crate::testing::Fixture;

fn body(rust: &str, method: &str) -> String {
    let mut fixture = Fixture::build(&[("lib.rs", rust)]);
    fixture.translated_method("lib.rs", method)
}

/// The call borrows and the body releases: `invokeRef` for an `Fn`/`FnMut`
/// bound, and `dropOwned` at the end, which is where Rust drops the parameter.
#[test]
fn a_by_value_fn_parameter_is_called_by_reference_and_released_by_the_body() {
    for bound in ["Fn(u32) -> u32", "FnMut(u32) -> u32"] {
        let ts = body(
            &format!("pub fn take<F: {bound}>(mut f: F, n: u32) -> u32 {{ f(n) }}"),
            "take",
        );
        assert!(ts.contains("invokeRef(f, n)"), "{bound}: the call borrows:\n{ts}");
        assert!(ts.contains("dropOwned(f)"), "{bound}: the body releases:\n{ts}");
    }
}

/// A bound that is `FnOnce` and nothing else is consumed by the call itself,
/// and `invoke` is what releases it. A second release would be a double drop.
#[test]
fn an_fn_once_parameter_is_released_by_the_call() {
    let ts = body(
        "pub fn take<F: FnOnce(u32) -> u32>(f: F, n: u32) -> u32 { f(n) }",
        "take",
    );
    assert!(ts.contains("invoke(f, n)"), "{ts}");
    assert!(!ts.contains("dropOwned(f)"), "the call already released it:\n{ts}");
}

/// A parameter written `&F` or `&mut F` is somebody else's: nothing in this
/// body may release it.
#[test]
fn a_borrowed_callable_parameter_is_left_to_its_owner() {
    for written in ["&F", "&mut F"] {
        let ts = body(
            &format!(
                "pub fn take<F: FnMut(u32) -> u32>(f: {written}, n: u32) -> u32 {{ f(n) }}"
            ),
            "take",
        );
        assert!(ts.contains("invokeRef(f, n)"), "{written}: {ts}");
        assert!(!ts.contains("dropOwned(f)"), "{written}: not this body's:\n{ts}");
    }
}

/// A body that hands the closure on releases nothing: the ordinary disposition
/// analysis decides, as it does for any other owned parameter.
#[test]
fn a_callable_parameter_handed_on_is_not_released_here() {
    let ts = body(
        "pub fn keep<F: Fn(u32) -> u32>(f: F) -> Box<dyn Fn(u32) -> u32>\n\
         where F: 'static { Box::new(f) }",
        "keep",
    );
    assert!(!ts.contains("dropOwned(f)"), "it was handed away:\n{ts}");
}

/// §4.9's rule — a type parameter whose only bound is a callable one is written
/// as the callable, because TypeScript infers nothing through a type parameter
/// constrained by a union — tested the WRITTEN type, so `&F` and `&mut F` were
/// missed. ankql's `Predicate::walk` kept `<F extends Invocable<..>>` and
/// answered `unknown` at six sites.
#[test]
fn the_callable_spelling_reaches_a_reference_parameter() {
    for written in ["F", "&F", "&mut F"] {
        let mut fixture = Fixture::build(&[(
            "lib.rs",
            &format!(
                "pub fn walk<T, F: FnMut(T) -> T>(start: T, f: {written}) -> T {{ f(start) }}"
            ),
        )]);
        let ts = fixture.emitted("lib.rs");
        assert!(
            ts.contains("f: Invocable<[T], T>"),
            "{written}: the parameter is written as the callable:\n{ts}"
        );
        assert!(
            !ts.contains("F extends Invocable"),
            "{written}: and the type parameter is gone:\n{ts}"
        );
    }
}
