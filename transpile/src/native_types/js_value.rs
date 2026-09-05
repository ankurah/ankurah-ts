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

pub fn translate(receiver: &str, rust_method: &str, args: &[String]) -> MethodTranslation {
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
        "is_object" => format!(
            "({r} !== null && typeof {r} === 'object')",
            r = receiver
        ),
        "is_truthy" => format!("(!!{})", receiver),
        "is_falsy" => format!("(!{})", receiver),
        "js_typeof" => format!("typeof {}", receiver),
        // `serde_json::Value::take` leaves `Null` behind and hands the value
        // over; a JavaScript value is not owned that way and there is nothing
        // to leave behind.
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

    fn expr(method: &str) -> String {
        match translate("value", method, &[]) {
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
        match translate("value", "dyn_into", &[]) {
            MethodTranslation::Refused { message, .. } => {
                assert!(message.contains("does not name"), "{message}");
            }
            _ => panic!("a cast the port cannot check has to be refused"),
        }
    }

    #[test]
    fn anything_else_passes_through() {
        assert!(matches!(
            translate("value", "some_method_nobody_declared", &[]),
            MethodTranslation::Passthrough
        ));
    }
}
