//! `to_string()` and `to_owned()`: a value of the receiver's own type, owned
//! rather than borrowed.
//!
//! Split out of `convert.rs`, which was over the 600-line rule. What is here is
//! one question — how the port COPIES a value — kept apart from the impl-table
//! questions `into` and `try_into` ask.

use crate::body::BodyTranslator;
use crate::ty::Ty;

impl BodyTranslator<'_> {
    /// `to_string()` and `to_owned()`: a value of the receiver's own type,
    /// owned rather than borrowed.
    ///
    /// The port maps `String` and `&str` to one type, so `s.to_string()` on a
    /// string is the string — `'Alice'.toString()` was a call whose only effect
    /// was to be there. Everything else keeps a real copy: `to_string` through
    /// `Display`, `to_owned` through `Clone`.
    pub(crate) fn owned_copy(
        &self,
        method: &str,
        from: Option<&Ty>,
        receiver: &str,
        span: proc_macro2::Span,
    ) -> Option<String> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let Some(from) = from else {
            self.fallback(
                span,
                format!(
                    "`{}` is written on a receiver the engine could not type, so the copy is \
                     written by the method's name alone",
                    method
                ),
            );
            // `to_owned` on an untyped receiver keeps the `clone` it was
            // written as before this: a string is the one type it is wrong for,
            // and a string is what the engine names most reliably.
            return (method == "to_owned").then(|| format!("{}.clone()", receiver));
        };
        use crate::name_map::shape::{js_shape, JsShape};
        // A reference is erased in emission, so `&str` and `String` are one
        // question here and `&Vec<u8>` and `Vec<u8>` are another.
        match js_shape(tc.registry, from.peel_refs()) {
            JsShape::Str => Some(receiver.to_string()),
            // A number and a bigint each carry `toString`, and a class that
            // implements `Display` is emitted with one, so the ordinary method
            // call is what `to_string` wants.
            _ if method == "to_string" => None,
            // A number, a bigint and a boolean are copied by being read: there
            // is nothing to clone and no `clone` on them to call, and
            // `n.clone()` was a TypeError at run time.
            JsShape::Number | JsShape::BigInt | JsShape::Boolean => Some(receiver.to_string()),
            JsShape::Bytes => Some(format!("{}.slice()", receiver)),
            // An array is copied element by element, by the element's own Clone
            // shape: `[...xs]` copies the ARRAY and leaves both copies holding
            // the same elements, which in the port is two owners for one value.
            JsShape::Array(inner) => {
                let element = crate::native_types::array::Element::of(tc.registry, &inner);
                match crate::native_types::array::copy(receiver, &element, &format!("[...{}]", receiver)) {
                    Ok(written) => Some(written),
                    Err(why) => {
                        self.fallback(
                            span,
                            format!("`{}` copies an array, which clones each element, and {}", method, why),
                        );
                        Some(format!("[...{}]", receiver))
                    }
                }
            }
            // `ToOwned` for everything else in the corpus is `Clone`, and the
            // emitted class carries `clone`.
            _ => Some(format!("{}.clone()", receiver)),
        }
    }
}
