//! What a pattern ASKS, as distinct from what it writes.
//!
//! For: two questions come up wherever a pattern is lowered — can this pattern
//! fail, and does it put any name anywhere — and the answers decide which
//! lowering the caller writes. `is_irrefutable` decides whether a position can
//! be a plain binding or needs a test beside it. `binds_nothing` decides
//! whether a name is written at all, and (in a consuming match) whether the arm
//! took the payload or left it for the arm to release.
//!
//! They are pure questions about syntax: no translator state, no types, no
//! output. They live apart from `patterns.rs` — which writes the destructuring
//! and the test — because a caller that only needs to know the shape of a
//! pattern should not have to read the writer to find out.

use super::BodyTranslator;

impl BodyTranslator<'_> {
    /// Does this pattern match whatever it is given?
    ///
    /// A name and a `_` take the value and always match; every other pattern
    /// asks a question of it. Callers use the answer to decide whether a
    /// position can be written as a binding alone, or needs a test written
    /// beside it so that the question still gets asked.
    pub(crate) fn is_irrefutable(pat: &syn::Pat) -> bool {
        match pat {
            // `None` is not a binding. syn hands it over as an identifier
            // because it is written without a path, and Rust resolves it to
            // `Option`'s empty case — binding it is an error, not a shadow.
            // `pattern_test` was given this exception and this was not, so a
            // `None` NESTED in any pattern — `Some(None)`, `E::Opt(None)` —
            // was read as a name that matches everything, and the arm ran for
            // a value that was there.
            syn::Pat::Ident(ident) if ident.ident == "None" && ident.subpat.is_none() => false,
            // `x @ Some(_)` binds *and* asks.
            syn::Pat::Ident(ident) => ident
                .subpat
                .as_ref()
                .map(|(_, inner)| Self::is_irrefutable(inner))
                .unwrap_or(true),
            syn::Pat::Wild(_) => true,
            syn::Pat::Reference(r) => Self::is_irrefutable(&r.pat),
            syn::Pat::Paren(p) => Self::is_irrefutable(&p.pat),
            syn::Pat::Type(t) => Self::is_irrefutable(&t.pat),
            syn::Pat::Tuple(t) => t.elems.iter().all(Self::is_irrefutable),
            _ => false,
        }
    }

    /// Does this pattern take no name out of the value at all?
    ///
    /// Rust's `_` is not a name: it says "there is a value here and I want
    /// nothing from it", and two of them in one pattern are two nothings.
    /// TypeScript has no such spelling — writing `_` there declares a variable
    /// called `_`, so `(Some(_), None)` emitted two `const _` in one block and
    /// `Comparison { left, operator: _, right: _ }` two `_` keys, and a
    /// JavaScript engine refuses the whole module. Every caller that would
    /// write a name asks this first and writes nothing instead.
    pub(crate) fn binds_nothing(pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Wild(_) => true,
            // `None` for the same reason `is_irrefutable` has it: syn hands it
            // over as an identifier because it is written without a path, and
            // Rust resolves it to `Option`'s empty case. It is a test, not a
            // name, so an arm that writes `Some(None)` took nothing out of the
            // payload and still owes the release.
            syn::Pat::Ident(ident) if ident.ident == "None" && ident.subpat.is_none() => true,
            // A pattern that is only a TEST takes no name either. `Lit::Flag(true)`
            // asks a question of the payload and puts nothing anywhere, so the arm
            // owes a release for the whole payload — and answering `false` here
            // said the arm had taken it, which is why `Ex::Literal(Lit::Flag(true))`
            // released nothing at all. `Pat::Path` is a unit variant or a const
            // (`None`, `Ordering::Less`); `Pat::Rest` is the `..` in a tuple.
            syn::Pat::Lit(_) | syn::Pat::Range(_) | syn::Pat::Path(_) | syn::Pat::Rest(_) => true,
            // Rust requires every alternative of an or-pattern to bind the same
            // names, so one alternative answers for all of them — but reading
            // them all costs nothing and says so.
            syn::Pat::Or(or) => or.cases.iter().all(Self::binds_nothing),
            syn::Pat::Reference(r) => Self::binds_nothing(&r.pat),
            syn::Pat::Paren(p) => Self::binds_nothing(&p.pat),
            syn::Pat::Type(t) => Self::binds_nothing(&t.pat),
            syn::Pat::Tuple(t) => t.elems.iter().all(Self::binds_nothing),
            syn::Pat::Slice(sl) => sl.elems.iter().all(Self::binds_nothing),
            syn::Pat::Struct(st) => st.fields.iter().all(|f| Self::binds_nothing(&f.pat)),
            syn::Pat::TupleStruct(ts) => ts.elems.iter().all(Self::binds_nothing),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BodyTranslator;

    /// `syn::Pat` has no `Parse` of its own — a pattern is ambiguous on its own,
    /// so syn asks the caller which flavour it wants. Reading one out of a
    /// written `match` arm is the flavour every caller here uses.
    fn pat(src: &str) -> syn::Pat {
        let arm: syn::ExprMatch = syn::parse_str(&format!("match x {{ {src} => 0, _ => 1 }}"))
            .unwrap_or_else(|e| panic!("cannot parse `{src}` as a match arm: {e}"));
        arm.arms.into_iter().next().expect("the arm just written").pat
    }

    /// D1: a pattern that is only a TEST binds nothing, so a consuming arm that
    /// writes one still owes a release for the whole payload. `Pat::Lit` fell to
    /// `_ => false` and the arm released nothing at all.
    #[test]
    fn a_pattern_that_only_tests_binds_nothing() {
        for src in [
            "true",
            "3",
            "1..=9",
            "None",
            "Ordering::Less",
            "Lit::Flag(true)",
            "Lit::Flag(_)",
            "Ex::Literal(Lit::Flag(true))",
            "Point { x: 0, .. }",
            "(true, 3)",
            "true | false",
            "Lit::Flag(true) | Lit::Count(0)",
        ] {
            assert!(
                BodyTranslator::binds_nothing(&pat(src)),
                "`{src}` takes no name out of the value and binds_nothing said it did"
            );
        }
    }

    /// The other direction, so the widening did not swallow everything: a
    /// pattern with a name in it anywhere binds something.
    #[test]
    fn a_pattern_with_a_name_anywhere_binds_something() {
        for src in [
            "n",
            "Lit::Count(n)",
            "Ex::Literal(Lit::Flag(b))",
            "(true, n)",
            "Point { x: 0, y }",
            "Lit::Flag(true) | Lit::Count(n)",
        ] {
            assert!(
                !BodyTranslator::binds_nothing(&pat(src)),
                "`{src}` puts a name somewhere and binds_nothing said it did not"
            );
        }
    }

    /// A test-only pattern is still REFUTABLE, which is what keeps its variant
    /// test from being dropped along with the binding (A1.1/A1.2's fix).
    #[test]
    fn a_pattern_that_only_tests_is_still_refutable() {
        for src in ["true", "3", "None", "Lit::Flag(_)", "Lit::Flag(true) | Lit::Count(0)"] {
            assert!(
                !BodyTranslator::is_irrefutable(&pat(src)),
                "`{src}` asks a question of the value and is_irrefutable said it did not"
            );
        }
    }
}
