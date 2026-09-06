//! An or-pattern whose alternatives bind their names in a form the translator
//! cannot read back, as the R12 hole it is.

use super::BodyTranslator;

/// An or-pattern whose alternatives bind their names in a form the translator
/// cannot read back, as the R12 hole it is.
///
/// PREMISE CHANGED 2026-09-05 (fixpass4 item 6): what stood here was `false` —
/// an arm written as one that never matches. That is a wrong answer twice over.
/// The branch is SKIPPED, so the program carries on as though the pattern had
/// not matched (core's `watcherset.ts` never registered an index watcher), and
/// the skipped branch still carried its own releases, naming bindings nothing
/// declared: `if (false) { .. } finally { literal.drop() }` is a
/// `ReferenceError` waiting for the day the test stops being `false`.
///
/// PREMISE CHANGED 2026-09-05 (fixpass6 item 4, D2): the hole used to stand
/// where the TEST goes, so the branch refused for every value the match was
/// given — including the ones whose pattern does not match, which Rust answers
/// with an empty `else`. R12's own wording is that the hole throws where the
/// BRANCH would have run. The test is the honest disjunction now, and this
/// writes the branch: each name the body reads is declared from a hole, so the
/// first statement of the branch throws and the emitted text is still one a
/// JavaScript engine loads and TypeScript types (`unsupported` answers
/// `never`).
pub(super) fn unreadable_alternatives(t: &BodyTranslator, or: &syn::PatOr) -> String {
    let what = "the alternatives of this pattern bind their names in a form the translator \
                cannot read back — each alternative has to bind the same names, one `const` \
                apiece — so this branch is a hole";
    t.fallback(syn::spanned::Spanned::span(or), what);
    let hole = crate::body::hole_text(what);
    let mut declared: Vec<String> = Vec::new();
    let mut bind = String::new();
    for case in &or.cases {
        for name in crate::body::pattern_names(case) {
            if declared.contains(&name) {
                continue;
            }
            // `as any` because `unsupported` answers `never`, and a `never`
            // name refuses every member the branch's body reads off it —
            // `a.n`, `a.drop()`. The branch throws at its first statement, so
            // the type is never observed; what it has to do is let the rest of
            // the branch through the type checker.
            bind.push_str(&format!("const {} = {} as any;\n", name, hole));
            declared.push(name);
        }
    }
    bind
}
