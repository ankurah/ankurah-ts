//! Conversions the runtime performs, rather than an emitted impl.
//!
//! For: `impl From<&str> for String` and `impl From<u32> for u64` are the
//! declared surface's, not the corpus's. There is no emitted class to hang a
//! `fromStr` on and no function to call: what the port writes is whatever
//! turning one JavaScript shape into another takes, which for most of these
//! pairs is nothing at all.
//!
//! The table is by *shape*, not by type name, because that is what decides the
//! text: `u8` to `u32` is one number to another and `u32` to `u64` is a number
//! to a bigint, and neither fact is readable from the pair of Rust names.

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// Type-erased conversion methods — apply to any receiver type.
///
/// `Formatter::alternate` is here because it is a question about a formatting
/// flag TypeScript has no notion of, and the answer is the same whatever it is
/// asked of.
pub fn translate(_receiver: &str, method: &str, _args: &[String]) -> Option<String> {
    match method {
        // Formatter::alternate() — TS has no alternate formatting flag
        "alternate" => Some("false".to_string()),

        _ => None,
    }
}

/// What the port writes to turn a value of one type into a value of another,
/// where the conversion is the runtime's own.
///
/// `None` means this pair is not in the table: the caller reports it rather
/// than writing a conversion nobody vetted.
pub fn between(reg: &TypeRegistry, from: &Ty, to: &Ty, value: &str) -> Option<String> {
    // `Box<T>` and `&T` are the value they hold, so a conversion into or out of
    // one is decided by what is inside it.
    if let JsShape::SameAs(inner) = js_shape(reg, from) {
        return between(reg, &inner, to, value);
    }
    if let JsShape::SameAs(inner) = js_shape(reg, to) {
        return between(reg, from, &inner, value);
    }
    match (js_shape(reg, from), js_shape(reg, to)) {
        // `&str` to `String`, `char` to `String`: one type in the port, so the
        // conversion is the value.
        (JsShape::Str, JsShape::Str) => Some(value.to_string()),
        (JsShape::Number, JsShape::Number) => Some(value.to_string()),
        (JsShape::Boolean, JsShape::Boolean) => Some(value.to_string()),
        // A widening into a 64-bit integer crosses from `number` to `bigint`,
        // which JavaScript will not do on its own: `1n + 1` throws.
        (JsShape::Number, JsShape::BigInt) => Some(format!("BigInt({})", value)),
        (JsShape::Boolean, JsShape::Number) => Some(format!("Number({})", value)),
        (JsShape::Boolean, JsShape::BigInt) => Some(format!("BigInt({})", value)),
        // `&[T]` to `Vec<T>` copies in Rust, and the copy is what the caller
        // then owns.
        (JsShape::Bytes, JsShape::Bytes) => Some(format!("{}.slice()", value)),
        (JsShape::Array(_), JsShape::Array(_)) => Some(format!("[...{}]", value)),
        // `Arc::from(v)` and `Rc::from(v)` wrap a value the way `new` does —
        // `Arc.from` is not a function, and the call raised a `TypeError`.
        (_, JsShape::Rc(name)) => Some(format!("{}.new({})", name, value)),
        _ => None,
    }
}
