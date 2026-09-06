//! An item written INSIDE a body.
//!
//! Rust lets a body declare a `const`, a `fn`, a `struct` or a `use`. The port
//! emits the `const` where it stands, because the rest of the body reads its
//! name; a `use` is scoped by the registry (`extract/uses.rs`); and every other
//! item is a declaration the port emits at module level or not at all.

use super::BodyTranslator;

impl BodyTranslator<'_> {
    /// A `const` written INSIDE a body is a name the rest of the body
    /// reads, and nothing emitted it: `const MAX_RETRIES: usize = 5;` in
    /// `Entity::commit` left `MAX_RETRIES` undeclared, which the range
    /// lowering turned from a name inside a comment into a
    /// `ReferenceError`. A `use` is scoped by the registry (see
    /// `extract/uses.rs`) and every other item in a body — a nested
    /// `fn`, a `struct` — is a declaration the port emits at module
    /// level or not at all.
    pub(crate) fn body_const(&self, c: &syn::ItemConst) -> String {
        let annotated = self
            .types
            .as_ref()
            .and_then(|tc| self.quietly(|| tc.borrow().resolve_written_type(&c.ty).ok()));
        let value = self.expecting(&c.expr, annotated.as_ref(), || self.expr_value(&c.expr));
        // J2: the annotation types the NAME too, not only the literal beside
        // it. Left unbound, `const MAX_RETRIES: usize = 3;` was a name nothing
        // could type, so `for attempt in 0..MAX_RETRIES` reached the range rule
        // with an endpoint it could not name — and the rule let an endpoint it
        // could not name through, because refusing one would have taken out
        // that very loop. With the annotation read, the endpoint has a width
        // and the rule can hold for every one it CAN name.
        if let (Some(ty), Some(tc)) = (annotated, self.types.as_ref()) {
            tc.borrow_mut().bind(&c.ident.to_string(), ty);
        }
        format!("const {} = {};\n", c.ident, value)
    }
}
