//! `serde_json::Value` and `wasm_bindgen::JsValue` — the values JavaScript
//! already has.
//!
//! Both are `unknown` in the port, so every question Rust asks of one is a
//! question JavaScript answers with `typeof`, `instanceof` or a comparison.
//! `value.as_f64()` is `Option<f64>` in Rust and so answers `null` here where
//! the value is not a number; `is_null` and `is_undefined` are the two halves
//! JavaScript's `== null` runs together, and each is written separately because
//! `ankql/src/conversion.rs` distinguishes them.
//!
//! A cast — `dyn_into`, `unchecked_into` — carries no run-time check the port
//! can write without knowing the target, so it is the value itself and the
//! caller's own `instanceof` test is what decides. That is a divergence from
//! `dyn_into`, which checks; it is recorded in spec 7a.

use super::MethodTranslation;
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// Which of the two `unknown`s this receiver is.
///
/// The port writes `serde_json::Value` and `wasm_bindgen::JsValue` with one
/// TypeScript spelling, and they are NOT the same type: `Value::is_object()` is
/// false for an array and `JsValue::is_object()` is true, `Value::take()`
/// leaves `Null` behind and `JsValue` has no such method, and `Value::clone()`
/// copies the document. Reading the spelling made them one, and a receiver the
/// engine could not type at all — `Ty::Infer` is `Unknown` too — was given
/// `serde_json`'s answers with nothing said.
#[derive(PartialEq, Clone, Copy)]
enum Which {
    SerdeJson,
    JsValue,
    /// The engine could not name this receiver's type.
    Unnamed,
}

fn which(reg: &TypeRegistry, ty: &Ty) -> Which {
    let Some(id) = ty.peel_refs().id() else {
        return Which::Unnamed;
    };
    // By the path the std surface declares it under, not by its leaf: the leaf
    // of `serde_json::Value` is `Value`, which is also what a crate type called
    // `Value` is — and core declares one.
    if reg.system_type("serde_json::Value") == Some(id) {
        return Which::SerdeJson;
    }
    if reg.system_type("wasm_bindgen::JsValue") == Some(id) {
        return Which::JsValue;
    }
    Which::Unnamed
}

pub fn translate(
    reg: &TypeRegistry,
    receiver_ty: &Ty,
    receiver: &str,
    rust_method: &str,
    args: &[String],
) -> MethodTranslation {
    let which = which(reg, receiver_ty);
    // A receiver nothing named is not evidence that it is a `serde_json::Value`.
    // Only the questions both types answer the same way are written for it; the
    // rest say so.
    if which == Which::Unnamed
        && matches!(rust_method, "is_object" | "take" | "clone" | "to_owned")
    {
        return MethodTranslation::Refused {
            message: format!(
                "`{}` is written here on a value the engine could not name, and \
                 `serde_json::Value` and `wasm_bindgen::JsValue` answer it differently — an \
                 array is not an object to serde_json, and only serde_json's `take` leaves \
                 `Null` behind",
                rust_method
            ),
            fallback: Box::new(MethodTranslation::Expr(receiver.to_string())),
        };
    }
    let written = match rust_method {
        // The three `as_*` accessors: a value of that kind, or nothing.
        "as_string" => format!("(typeof {r} === 'string' ? {r} : null)", r = receiver),
        "as_bool" => format!("(typeof {r} === 'boolean' ? {r} : null)", r = receiver),
        "as_f64" => format!("(typeof {r} === 'number' ? {r} : null)", r = receiver),
        "as_i64" => format!(
            "(typeof {r} === 'number' && Number.isInteger({r}) ? BigInt({r}) : null)",
            r = receiver
        ),
        "as_u64" => format!(
            "(typeof {r} === 'number' && Number.isInteger({r}) && {r} >= 0 ? BigInt({r}) : null)",
            r = receiver
        ),
        "as_array" => format!("(Array.isArray({r}) ? {r} : null)", r = receiver),
        "as_object" => format!(
            "({r} !== null && typeof {r} === 'object' && !Array.isArray({r}) ? {r} : null)",
            r = receiver
        ),
        // The predicates.
        "is_null" => format!("({} === null)", receiver),
        "is_undefined" => format!("({} === undefined)", receiver),
        "is_string" => format!("(typeof {} === 'string')", receiver),
        "is_boolean" => format!("(typeof {} === 'boolean')", receiver),
        "is_number" => format!("(typeof {} === 'number')", receiver),
        "is_function" => format!("(typeof {} === 'function')", receiver),
        "is_array" => format!("Array.isArray({})", receiver),
        // `serde_json::Value::is_object()` is false for an array: an array is
        // `Value::Array`, a different variant. `JsValue::is_object()` is
        // JavaScript's own question, and an array IS an object there.
        "is_object" if which == Which::SerdeJson => format!(
            "({r} !== null && typeof {r} === 'object' && !Array.isArray({r}))",
            r = receiver
        ),
        "is_object" => format!("({r} !== null && typeof {r} === 'object')", r = receiver),
        "is_truthy" => format!("(!!{})", receiver),
        "is_falsy" => format!("(!{})", receiver),
        "js_typeof" => format!("typeof {}", receiver),
        // `serde_json::Value::take` takes the value out and leaves `Null` in
        // its place, and `clone`/`to_owned` copy the whole document. Writing
        // the receiver for all three made the two handles ALIAS: a mutation
        // through either was visible through both, and a `take` left the
        // source holding what it was supposed to have given away.
        "take" if which == Which::SerdeJson => format!(
            "(() => {{ const _v = {r}; {r} = null; return _v; }})()",
            r = receiver
        ),
        "clone" | "to_owned" if which == Which::SerdeJson => {
            format!("structuredClone({})", receiver)
        }
        // A `JsValue` is a handle to something JavaScript owns; there is
        // nothing to leave behind and nothing to copy.
        "take" | "clone" | "to_owned" => receiver.to_string(),
        // A cast the port cannot check: `dyn_into::<Uint8Array>()` names the
        // target in a turbofish emission does not keep.
        "dyn_into" | "unchecked_into" | "dyn_ref" | "unchecked_ref" => {
            return MethodTranslation::Refused {
                message: format!(
                    "`{}` casts a JavaScript value to a type the emitted call does not name, \
                     so the value stands as it is and nothing checks it",
                    rust_method
                ),
                fallback: Box::new(MethodTranslation::Expr(receiver.to_string())),
            };
        }
        "is_instance_of" => {
            return MethodTranslation::Refused {
                message: "`is_instance_of` names the type it tests in a turbofish, which \
                          emission does not keep, so the test cannot be written"
                    .to_string(),
                fallback: Box::new(MethodTranslation::Expr(format!(
                    "(typeof {} === 'object')",
                    receiver
                ))),
            };
        }
        // `serde_json::Value`'s indexing.
        "get" if args.len() == 1 => format!(
            "(({r} as Record<string, unknown>)?.[{a}] ?? null)",
            r = receiver,
            a = args[0]
        ),
        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    /// A fixture whose registry holds the std surface, so the two `unknown`s
    /// can be told apart by their identity — which is the whole point of this
    /// module now.
    fn fixture() -> Fixture {
        Fixture::build(&[("lib.rs", "pub fn f() {}")])
    }

    fn value_ty(f: &Fixture, path: &str) -> crate::ty::Ty {
        f.system(path, Vec::new())
    }

    fn expr(method: &str) -> String {
        let f = fixture();
        let ty = value_ty(&f, "serde_json::Value");
        match translate(&f.reg, &ty, "value", method, &[]) {
            MethodTranslation::Expr(s) => s,
            other => match other {
                MethodTranslation::Refused { fallback, .. } => match *fallback {
                    MethodTranslation::Expr(s) => s,
                    _ => panic!("no expression for {method}"),
                },
                _ => panic!("no expression for {method}"),
            },
        }
    }

    #[test]
    fn the_accessors_are_type_guards() {
        // `ankql/src/conversion.rs` builds a Literal from a JS scalar this way.
        assert_eq!(expr("as_string"), "(typeof value === 'string' ? value : null)");
        assert_eq!(expr("as_bool"), "(typeof value === 'boolean' ? value : null)");
        assert_eq!(expr("as_f64"), "(typeof value === 'number' ? value : null)");
    }

    #[test]
    fn null_and_undefined_are_two_questions() {
        assert_eq!(expr("is_null"), "(value === null)");
        assert_eq!(expr("is_undefined"), "(value === undefined)");
    }

    #[test]
    fn an_unwritable_cast_is_refused_and_stands_as_the_value() {
        let f = fixture();
        let ty = value_ty(&f, "serde_json::Value");
        match translate(&f.reg, &ty, "value", "dyn_into", &[]) {
            MethodTranslation::Refused { message, .. } => {
                assert!(message.contains("does not name"), "{message}");
            }
            _ => panic!("a cast the port cannot check has to be refused"),
        }
    }

    #[test]
    fn anything_else_passes_through() {
        let f = fixture();
        let ty = value_ty(&f, "serde_json::Value");
        assert!(matches!(
            translate(&f.reg, &ty, "value", "some_method_nobody_declared", &[]),
            MethodTranslation::Passthrough
        ));
    }

    /// `serde_json::Value::is_object()` is FALSE for an array — an array is
    /// `Value::Array`, a different variant — and `JsValue::is_object()` is
    /// JavaScript's own question, where an array IS an object. One spelling,
    /// two types, two answers.
    #[test]
    fn is_object_answers_differently_for_the_two_unknowns() {
        let f = fixture();
        let serde = value_ty(&f, "serde_json::Value");
        let js = value_ty(&f, "wasm_bindgen::JsValue");
        let written = |ty: &crate::ty::Ty| match translate(&f.reg, ty, "value", "is_object", &[]) {
            MethodTranslation::Expr(s) => s,
            _ => panic!("is_object has an expression"),
        };
        assert!(written(&serde).contains("!Array.isArray(value)"), "{}", written(&serde));
        assert!(!written(&js).contains("!Array.isArray(value)"), "{}", written(&js));
    }

    /// `Value::take()` takes the value out and leaves `Null`; `clone` copies
    /// the document. Writing the receiver for both made the two handles alias.
    #[test]
    fn take_and_clone_are_real_for_serde_json() {
        let f = fixture();
        let serde = value_ty(&f, "serde_json::Value");
        let js = value_ty(&f, "wasm_bindgen::JsValue");
        let written = |ty: &crate::ty::Ty, m: &str| match translate(&f.reg, ty, "value", m, &[]) {
            MethodTranslation::Expr(s) => s,
            _ => panic!("{m} has an expression"),
        };
        assert!(written(&serde, "take").contains("value = null"), "{}", written(&serde, "take"));
        assert_eq!(written(&serde, "clone"), "structuredClone(value)");
        // A `JsValue` is a handle to something JavaScript owns: nothing to
        // leave behind, nothing to copy.
        assert_eq!(written(&js, "take"), "value");
        assert_eq!(written(&js, "clone"), "value");
    }
}
