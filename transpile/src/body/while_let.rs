//! `while let PAT = e { .. }`: a loop that reads its scrutinee afresh each turn,
//! tests it against the pattern, and stops when it does not match.
//!
//! For: Rust evaluates the scrutinee once per TURN, binds the payload for that
//! turn only, and drops what the turn bound at the end of it. Written as an
//! expression the condition came out a comment and the binding was never
//! declared. What a turn that does NOT match owes is here too: the scrutinee
//! was read and the loop is leaving it behind, so the loop releases it.

use super::{bound_names, indent, BodyTranslator};
use crate::ownership;

impl BodyTranslator<'_> {
    /// `while let PAT = e { body }` as a loop that tests each turn.
    ///
    /// The scrutinee is read once per turn into a temporary, tested against the
    /// pattern, and its payload bound inside the body — which is what Rust does
    /// and what the previous emission, a comment in the condition, did not.
    pub(crate) fn while_let(
        &self,
        let_expr: &syn::ExprLet,
        body: &syn::Block,
        label: &str,
        written_label: &Option<syn::Label>,
    ) -> String {
        // The scrutinee is read afresh every turn, in value position: it is the
        // turn's own value, and an `if` written there is a run of statements
        // that `const _v = …` cannot hold. Whatever it lifted belongs inside
        // the loop with it, because it is taken again on the next turn.
        let (scrutinee, lifted) = self.with_own_hoists(|| self.expr_value(&let_expr.expr));
        let ty = self.borrowed_scrutinee_type(&let_expr.expr);
        let _bindings = self.enter_pattern(&let_expr.pat, ty.as_ref());
        // The pattern binds afresh each turn, and Rust drops what it bound at
        // the end of that turn — so the release goes inside the loop, not after
        // it.
        let owned = self.claim_bindings(&bound_names(&let_expr.pat), &body.stmts);
        let translated = crate::control_flow::sentinel::inside_a_loop(self, written_label, || {
            self.translate_loop_block(body)
        });
        drop(_bindings);

        let subject = self.fresh_temp();
        // The binding scope closed above, so the borrowed-ness of the value
        // this turn takes apart is said again here.
        let (test, bind) =
            self.matching(ty.as_ref(), || self.pattern_test(&subject, &let_expr.pat));
        let turn = self.wrap_bindings(&owned, translated);
        let leaving = self.abandoned_scrutinee(&let_expr.expr, &let_expr.pat, &subject);
        let read = ownership::hoisted(&format!("const {} = {};\n", subject, scrutinee), &lifted);
        format!(
            "{}for (;;) {{\n{}  if (!({})) {{\n{}    break;\n  }}\n{}{}}}",
            label,
            indent(&read),
            test,
            indent(&indent(&leaving)),
            indent(&bind),
            indent(&turn)
        )
    }

    /// What the turn owes for a scrutinee whose pattern did not match.
    ///
    /// Rust drops the value the turn read when no pattern took it apart, so the
    /// path that leaves the loop releases it. The path that *did* match is a
    /// different question: where the pattern took an owned payload out, that
    /// payload belongs to the binding from there and the enum it came out of
    /// has to be marked moved — which only `intoMatch` does, and an arrow
    /// function is not something a `break` can leave. That one is reported.
    fn abandoned_scrutinee(&self, expr: &syn::Expr, pat: &syn::Pat, subject: &str) -> String {
        // A nullable scrutinee is its own payload: `Option<T>` is `T | null`
        // here, so `Some(v)` binds the very value the turn read and the turn
        // that did not match read a `null`, which owns nothing. There is no
        // wrapper left over on either path.
        if self
            .quietly(|| self.resolve_expr_type(expr))
            .is_ok_and(|ty| self.is_nullable(&ty))
        {
            return String::new();
        }
        let Some(release) = self.release_of(expr, subject) else {
            return String::new();
        };
        if self.pattern_takes_a_payload(expr, pat) {
            self.fallback(
                syn::spanned::Spanned::span(expr),
                "this `while let` takes an owned payload out of the value it read, and the \
                 value it came out of is not marked moved, so nothing releases the rest of it",
            );
        }
        format!("{}\n", release)
    }
}
