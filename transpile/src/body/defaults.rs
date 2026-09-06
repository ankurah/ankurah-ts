//! `unwrap_or_default`, and the one thing it needs that the port does not carry.
//!
//! For: `Result<T, E>::unwrap_or_default()` and `Option<T>::unwrap_or_default()`
//! answer the value or `T::default()`, and the port carries no `Default` at all
//! — `#[serde(skip)]` is refused for the same reason. Written from the method's
//! NAME, both came out `x.unwrapOrDefault()`, which nothing declares: eleven
//! emitted sites, every one a `TypeError`, and five of them on a `string | null`
//! that has no members at all (N10). Where the port can name the default from
//! the resolved type it writes it; where it cannot, the site is a hole.

use super::BodyTranslator;
use crate::name_map::shape::{js_shape, JsShape};
use crate::ty::Ty;

impl BodyTranslator<'_> {
    /// The registry's own words for a type, for a diagnostic to name it.
    fn type_words(&self, ty: &Ty) -> String {
        match &self.types {
            Some(tc) => tc.borrow().registry.describe(ty),
            None => "this type".to_string(),
        }
    }

    /// What Rust's `Default` is for this type, as the port writes it — or
    /// nothing, where the port cannot say.
    ///
    /// Only the shapes whose default is the same in every instantiation: the
    /// empty string, zero, `false`, the empty sequence, and absence. A type
    /// with a `#[derive(Default)]` of its own has one Rust can name and the
    /// port cannot, because it emits no `default()` for it; a `HashMap` and a
    /// `HashSet` have a default the RUNTIME could build, and neither has a
    /// corpus site here, so both wait for one rather than being guessed at.
    fn default_text(&self, ty: &Ty) -> Option<String> {
        let tc = self.types.as_ref()?.borrow();
        // U8: a FIXED-SIZE array first, because the port's shape erases the
        // length. `[u32; 3]`'s default is three zeros and `[u8; 3]`'s is three
        // zero bytes; read through `js_shape` both came out empty — `value ?? []`
        // and `value ?? new Uint8Array()` — which is a wrong value rather than
        // a missing one. Written only where the length is a literal and the
        // element's own default is one of the shapes below; anything else
        // (a `const N` length, an element with a `Default` of its own) takes
        // the refusal, which is what it took before.
        if let Ty::Array { elem, len } = ty.peel_refs() {
            // A length the port cannot read is a refusal rather than an empty
            // collection: `[u32; N]`'s default is N zeroes whatever N is, and
            // the empty array is that only for N = 0.
            let crate::ty::ArrayLen::Lit(len) = len else { return None };
            let element = self.default_text(elem)?;
            let repeated = vec![element.as_str(); *len as usize];
            return Some(match js_shape(tc.registry, elem.peel_refs()) {
                // A byte array is a `Uint8Array` in the port, whose own
                // constructor makes N zeroes.
                JsShape::Number if matches!(elem.peel_refs(), Ty::Prim(crate::ty::Prim::U8)) => {
                    format!("new Uint8Array({})", len)
                }
                _ => format!("[{}]", repeated.join(", ")),
            });
        }
        Some(match js_shape(tc.registry, ty.peel_refs()) {
            JsShape::Str => "''".to_string(),
            JsShape::Number => "0".to_string(),
            JsShape::BigInt => "0n".to_string(),
            JsShape::Boolean => "false".to_string(),
            JsShape::Bytes => "new Uint8Array()".to_string(),
            JsShape::Array(_) => "[]".to_string(),
            JsShape::Nullable(_) => "null".to_string(),
            _ => return None,
        })
    }

    /// `r.unwrap_or_default()` and `o.unwrap_or_default()`, from the type the
    /// receiver resolved to.
    ///
    /// `Option<T>` is `T | null` here, so the answer is `??` and the value on
    /// its right is `T`'s default. A `Result` is an object with an `unwrapOr`
    /// of its own, which releases the wrapper as Rust's does. Anything else —
    /// including a `T` whose default the port cannot name — is refused rather
    /// than filled in with `undefined`.
    pub(crate) fn unwrap_or_default(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
    ) -> Option<String> {
        if rust_method != "unwrap_or_default" || !call.args.is_empty() {
            return None;
        }
        let refuse = |what: String| Some(self.hole(syn::spanned::Spanned::span(call), what));
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(&call.receiver)) else {
            return refuse(
                "`unwrap_or_default` answers the value or the type's `Default`, and the engine \
                 could not type this receiver, so it cannot say which default that is"
                    .to_string(),
            );
        };
        let Some(tc) = &self.types else { return None };
        let (payload, nullable) = {
            let tc = tc.borrow();
            match js_shape(tc.registry, ty.peel_refs()) {
                JsShape::Nullable(inner) => (Some(inner.clone()), true),
                _ => (BodyTranslator::result_payload(self, &ty), false),
            }
        };
        let Some(payload) = payload else {
            return refuse(format!(
                "`unwrap_or_default` is written on a `{}`, which the port reads as neither an \
                 `Option` nor a `Result`, so there is no payload whose default it could answer",
                self.type_words(&ty)
            ));
        };
        let Some(default) = self.default_text(&payload) else {
            return refuse(format!(
                "`unwrap_or_default` would answer `{}`'s `Default`, and the port carries no \
                 default for that type",
                self.type_words(&payload)
            ));
        };
        Some(match nullable {
            true => format!("({} ?? {})", receiver, default),
            false => format!("{}.unwrapOr({})", receiver, default),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    fn body(rust: &str, method: &str) -> String {
        let mut f = Fixture::build(&[("lib.rs", rust)]);
        f.translated_method("lib.rs", method)
    }

    /// N10: `unwrap_or_default` answers the payload type's `Default`, which the
    /// method's own NAME cannot say. `unwrapOrDefault` is declared nowhere, so
    /// every emitted site was a `TypeError` — and five of them stood on a
    /// `string | null`, which has no members whatever.
    #[test]
    fn unwrap_or_default_is_written_from_the_payload_type() {
        for (rust, want) in [
            ("pub fn f(s: Option<String>) -> String { s.unwrap_or_default() }", "(s ?? '')"),
            ("pub fn f(n: Option<u32>) -> u32 { n.unwrap_or_default() }", "(n ?? 0)"),
            ("pub fn f(b: Option<bool>) -> bool { b.unwrap_or_default() }", "(b ?? false)"),
            (
                "pub fn f(v: Option<Vec<u8>>) -> Vec<u8> { v.unwrap_or_default() }",
                "(v ?? new Uint8Array())",
            ),
            (
                "pub fn f(r: Result<Vec<u32>, String>) -> Vec<u32> { r.unwrap_or_default() }",
                "r.unwrapOr([])",
            ),
        ] {
            let ts = body(rust, "f");
            assert!(ts.contains(want), "`{}` for:\n{}", want, ts);
            assert!(!ts.contains("unwrapOrDefault"), "{}", ts);
        }
    }

    /// A payload whose default the port cannot name is a hole, not `undefined`.
    /// The port carries no `Default` at all — `#[serde(skip)]` is refused for
    /// the same reason.
    #[test]
    fn a_default_the_port_cannot_name_is_refused() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Held { pub n: u32 }\n\
             pub fn f(h: Option<Held>) -> Held { h.unwrap_or_default() }",
        )]);
        let ts = f.translated_method("lib.rs", "f");
        assert!(ts.contains("unsupported("), "{}", ts);
        assert!(
            f.messages().iter().any(|m| m.contains("carries no default for that type")),
            "and it says which type: {:?}",
            f.messages()
        );
    }
}
