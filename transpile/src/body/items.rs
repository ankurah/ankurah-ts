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
        let value = self.expecting(
            &c.expr,
            self.types
                .as_ref()
                .and_then(|tc| {
                    self.quietly(|| tc.borrow().resolve_written_type(&c.ty).ok())
                })
                .as_ref(),
            || self.expr_value(&c.expr),
        );
        format!("const {} = {};\n", c.ident, value)
    }
}
