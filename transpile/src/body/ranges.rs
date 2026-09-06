//! `a..b` as a VALUE.
//!
//! Rust's range is a value with methods on it — `for i in 0..n` iterates one and
//! `(0..n).rev()` calls a method on one — and the port has no `Range` type.

use super::BodyTranslator;

impl BodyTranslator<'_> {
    /// A range is a value in Rust — `for i in 0..n` iterates one and
    /// `(0..n).rev()` calls a method on one — and the port has no
    /// `Range` type. It used to be written `undefined`, so
    /// `for (const attempt of undefined)` raised `undefined is not
    /// iterable` the first time the loop was reached; `Entity::commit`'s
    /// retry loop is one of those. A BOUNDED range is the sequence of
    /// its values, which is what makes every adaptor on it work — `rev`,
    /// `map`, `filter` and `contains` are all array operations here.
    ///
    /// An unbounded one — `..n`, `a..`, `..` — has no sequence to build,
    /// and in the one position where Rust means a SLICE by it the index
    /// lowering has already answered. So it is a hole (R12).
    pub(crate) fn range_value(&self, range: &syn::ExprRange) -> String {
        let ends = (range.start.as_ref(), range.end.as_ref());
        let (Some(start), Some(end)) = ends else {
            return self.hole(
                syn::spanned::Spanned::span(range),
                "an unbounded range is not a sequence the port can build, and this one \
                 does not stand where a slice is taken",
            );
        };
        // A width the port holds in a `bigint` cannot be counted with
        // `n++` on a number, and a range of one is not a shape the
        // corpus writes. Refusing it is cheaper than a helper nothing
        // reaches.
        let bigint_end = [start, end].iter().any(|e| {
            self.quietly(|| self.resolve_expr_type(e)).is_ok_and(|ty| {
                matches!(
                    ty.peel_refs(),
                    crate::ty::Ty::Prim(
                        crate::ty::Prim::U64
                            | crate::ty::Prim::I64
                            | crate::ty::Prim::U128
                            | crate::ty::Prim::I128
                    )
                )
            })
        });
        if bigint_end {
            return self.hole(
                syn::spanned::Spanned::span(range),
                "a range over a width the port holds in a `bigint` is not a sequence the \
                 port builds",
            );
        }
        let from = self.expr_value(start);
        let to = self.expr_value(end);
        let helper = match range.limits {
            syn::RangeLimits::Closed(_) => "rangeIncl",
            syn::RangeLimits::HalfOpen(_) => "range",
        };
        format!("{}({}, {})", helper, from, to)
    }
}
